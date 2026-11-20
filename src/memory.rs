use std::mem::size_of;
use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{
    GetPerformanceInfo, GetProcessMemoryInfo, PERFORMANCE_INFORMATION, PROCESS_MEMORY_COUNTERS_EX,
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};

#[derive(Debug, Clone, Default)]
pub struct SystemMemorySummary {
    pub total_phys_bytes: u64,
    pub avail_phys_bytes: u64,
    pub used_phys_bytes: u64,
    pub memory_load_pct: u32,
    pub total_page_file_bytes: u64,
    pub avail_page_file_bytes: u64,
    pub commit_total_bytes: u64,
    pub commit_limit_bytes: u64,
    pub paged_pool_bytes: u64,
    pub nonpaged_pool_bytes: u64,
    pub system_cache_bytes: u64,
    pub compressed_bytes: u64,
    pub process_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessMemoryEntry {
    pub pid: u32,
    pub name: String,
    pub working_set_bytes: u64,
    pub peak_working_set_bytes: u64,
    pub private_bytes: u64,
    pub pagefile_bytes: u64,
    pub gpu_dedicated_bytes: u64,
    pub gpu_dedicated_requested_bytes: u64,
    pub gpu_shared_bytes: u64,
    pub gpu_shared_requested_bytes: u64,
    pub gpu_per_adapter: Vec<crate::gpu::AdapterGpuUsage>,
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

pub fn collect_system_summary() -> SystemMemorySummary {
    let mut summary = SystemMemorySummary::default();

    unsafe {
        let mut mem_status = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };

        if GlobalMemoryStatusEx(&mut mem_status).is_ok() {
            summary.total_phys_bytes = mem_status.ullTotalPhys;
            summary.avail_phys_bytes = mem_status.ullAvailPhys;
            summary.used_phys_bytes = mem_status.ullTotalPhys.saturating_sub(mem_status.ullAvailPhys);
            summary.memory_load_pct = mem_status.dwMemoryLoad;
            summary.total_page_file_bytes = mem_status.ullTotalPageFile;
            summary.avail_page_file_bytes = mem_status.ullAvailPageFile;
        }

        let mut perf_info = PERFORMANCE_INFORMATION {
            cb: size_of::<PERFORMANCE_INFORMATION>() as u32,
            ..Default::default()
        };

        if GetPerformanceInfo(&mut perf_info, size_of::<PERFORMANCE_INFORMATION>() as u32).is_ok() {
            let page_size = perf_info.PageSize as u64;
            summary.commit_total_bytes = (perf_info.CommitTotal as u64) * page_size;
            summary.commit_limit_bytes = (perf_info.CommitLimit as u64) * page_size;
            summary.paged_pool_bytes = (perf_info.KernelPaged as u64) * page_size;
            summary.nonpaged_pool_bytes = (perf_info.KernelNonpaged as u64) * page_size;
            summary.system_cache_bytes = (perf_info.SystemCache as u64) * page_size;
            summary.process_count = perf_info.ProcessCount;
        }
    }

    summary.compressed_bytes = collect_compressed_memory_bytes();
    summary
}

fn collect_compressed_memory_bytes() -> u64 {
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
    use windows::core::s;

    type NtQuerySystemInformationFn =
        unsafe extern "system" fn(u32, *mut u8, u32, *mut u32) -> i32;

    unsafe {
        let Ok(ntdll) = LoadLibraryA(s!("ntdll.dll")) else {
            return 0;
        };
        let Some(proc_addr) = GetProcAddress(ntdll, s!("NtQuerySystemInformation")) else {
            return 0;
        };
        let nt_query: NtQuerySystemInformationFn = std::mem::transmute(proc_addr);

        let mut buf_size = 512 * 1024u32;
        let mut buf = vec![0u8; buf_size as usize];
        loop {
            let mut needed = 0u32;
            let status = nt_query(5, buf.as_mut_ptr(), buf_size, &mut needed);
            if status == 0 {
                break;
            }
            if status == -1073741820_i32 {
                // STATUS_INFO_LENGTH_MISMATCH
                buf_size = needed.max(buf_size * 2);
                buf.resize(buf_size as usize, 0);
            } else {
                return 0;
            }
        }

        let mut offset = 0usize;
        loop {
            if offset + 128 > buf.len() {
                break;
            }
            let base = buf.as_ptr().add(offset);
            let next_offset = *(base as *const u32) as usize;
            // ImageName UNICODE_STRING at offset 56 (x64): Length(u16), MaxLength(u16), pad(4), Buffer(*const u16)
            let name_len = *(base.add(56) as *const u16) as usize;
            let name_ptr = *(base.add(64) as *const *const u16);
            if !name_ptr.is_null() && name_len > 0 {
                let slice = std::slice::from_raw_parts(name_ptr, name_len / 2);
                let name = String::from_utf16_lossy(slice);
                if name.eq_ignore_ascii_case("Memory Compression") {
                    // WorkingSetSize (SIZE_T) is at offset 120 on x64
                    return *(base.add(120) as *const usize) as u64;
                }
            }
            if next_offset == 0 {
                break;
            }
            offset += next_offset;
        }
    }
    0
}

pub fn collect_process_memory() -> Vec<ProcessMemoryEntry> {
    let mut entries = Vec::new();

    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(handle) if handle != INVALID_HANDLE_VALUE => handle,
            _ => return entries,
        };

        let mut pe = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut pe).is_ok() {
            loop {
                let pid = pe.th32ProcessID;
                if pid != 0 {
                    let name_len = pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len());
                    let name = String::from_utf16_lossy(&pe.szExeFile[..name_len]);

                    // Try to open process to query memory
                    // Prefer limited query, fallback to query info + vm read if needed
                    let process_handle = OpenProcess(
                        PROCESS_QUERY_LIMITED_INFORMATION,
                        false,
                        pid,
                    ).or_else(|_| {
                        OpenProcess(
                            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                            false,
                            pid,
                        )
                    });

                    if let Ok(handle) = process_handle
                        && !handle.is_invalid() {
                            let mut counters = PROCESS_MEMORY_COUNTERS_EX::default();
                            let struct_size = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;

                            if GetProcessMemoryInfo(
                                handle,
                                &mut counters as *mut _ as *mut _,
                                struct_size,
                            ).is_ok()
                            {
                                entries.push(ProcessMemoryEntry {
                                    pid,
                                    name,
                                    working_set_bytes: counters.WorkingSetSize as u64,
                                    peak_working_set_bytes: counters.PeakWorkingSetSize as u64,
                                    private_bytes: counters.PrivateUsage as u64,
                                    pagefile_bytes: counters.PagefileUsage as u64,
                                    gpu_dedicated_bytes: 0,
                                    gpu_dedicated_requested_bytes: 0,
                                    gpu_shared_bytes: 0,
                                    gpu_shared_requested_bytes: 0,
                                    gpu_per_adapter: Vec::new(),
                                });
                            }
                            let _ = CloseHandle(handle);
                        }
                }

                if Process32NextW(snapshot, &mut pe).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }

    // Default sort by Working Set descending
    entries.sort_by_key(|a| std::cmp::Reverse(a.working_set_bytes));

    entries
}
