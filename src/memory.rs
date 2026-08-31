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
    pub process_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessMemoryEntry {
    pub pid: u32,
    pub name: String,
    pub working_set_bytes: u64,
    pub private_bytes: u64,
    pub gpu_dedicated_bytes: u64,
    pub gpu_shared_bytes: u64,
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
            summary.process_count = perf_info.ProcessCount;
        }
    }

    summary
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
                                    private_bytes: counters.PrivateUsage as u64,
                                    gpu_dedicated_bytes: 0,
                                    gpu_shared_bytes: 0,
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
