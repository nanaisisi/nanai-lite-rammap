use std::collections::HashMap;
use std::ffi::c_void;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HMODULE, LUID};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL,
    IDXGIAdapter3, IDXGIFactory1,
};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::core::Interface;
use windows::core::s;

#[derive(Debug, Clone, Default)]
pub struct GpuAdapterInfo {
    pub name: String,
    pub luid: LUID,
    pub dedicated_vram_bytes: u64,
    pub shared_ram_bytes: u64,
    pub dedicated_usage_bytes: u64,
    pub dedicated_budget_bytes: u64,
    pub shared_usage_bytes: u64,
    pub shared_budget_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessGpuMemory {
    pub dedicated_bytes: u64,
    pub shared_bytes: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct D3DKMT_QUERYSTATISTICS_PROCESS_SEGMENT_GROUP_INFORMATION {
    pub budget: u64,
    pub requested: u64,
    pub usage: u64,
    pub demoted: u64,
}

const D3DKMT_QUERYSTATISTICS_PROCESS_SEGMENT_GROUP: u32 = 9;

#[repr(C)]
pub struct D3DKMT_QUERYSTATISTICS {
    pub r#type: u32,
    pub adapter_luid: LUID,
    pub process_handle: HANDLE,
    pub query_result: [u8; 320], // Union D3DKMT_QUERYSTATISTICS_RESULT (large enough for all variants)
    pub query_process_segment_group: u32, // SegmentGroup in union (0 = Local/Dedicated, 1 = NonLocal/Shared)
    pub reserved_union: [u32; 7],
}

type D3DKMTQueryStatisticsFn = unsafe extern "system" fn(p_data: *mut c_void) -> i32;

struct Gdi32D3DKmt {
    _module: HMODULE,
    query_statistics: Option<D3DKMTQueryStatisticsFn>,
}

unsafe impl Send for Gdi32D3DKmt {}
unsafe impl Sync for Gdi32D3DKmt {}

impl Gdi32D3DKmt {
    fn load() -> Option<Self> {
        unsafe {
            let module = LoadLibraryA(s!("gdi32.dll")).ok()?;
            let func_ptr = GetProcAddress(module, s!("D3DKMTQueryStatistics"));
            let query_statistics =
                func_ptr.map(|p| std::mem::transmute::<_, D3DKMTQueryStatisticsFn>(p));
            Some(Self {
                _module: module,
                query_statistics,
            })
        }
    }
}

pub fn collect_gpu_adapters() -> Vec<GpuAdapterInfo> {
    let mut adapters = Vec::new();

    unsafe {
        let factory: IDXGIFactory1 = match CreateDXGIFactory1() {
            Ok(f) => f,
            Err(_) => return adapters,
        };

        let mut i = 0u32;
        while let Ok(adapter) = factory.EnumAdapters1(i) {
            i += 1;
            let desc = match adapter.GetDesc1() {
                Ok(d) => d,
                Err(_) => continue,
            };

            // Skip software adapter / Microsoft Basic Render Driver
            if (desc.Flags & 2) != 0 {
                continue;
            }

            let name_len = desc
                .Description
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(desc.Description.len());
            let name = String::from_utf16_lossy(&desc.Description[..name_len]);

            let mut info = GpuAdapterInfo {
                name,
                luid: desc.AdapterLuid,
                dedicated_vram_bytes: desc.DedicatedVideoMemory as u64,
                shared_ram_bytes: desc.SharedSystemMemory as u64,
                ..Default::default()
            };

            // Query IDXGIAdapter3 for budget and current usage
            if let Ok(adapter3) = adapter.cast::<IDXGIAdapter3>() {
                let mut local_info = Default::default();
                if adapter3
                    .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut local_info)
                    .is_ok()
                {
                    info.dedicated_budget_bytes = local_info.Budget;
                    info.dedicated_usage_bytes = local_info.CurrentUsage;
                }

                let mut non_local_info = Default::default();
                if adapter3
                    .QueryVideoMemoryInfo(
                        0,
                        DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL,
                        &mut non_local_info,
                    )
                    .is_ok()
                {
                    info.shared_budget_bytes = non_local_info.Budget;
                    info.shared_usage_bytes = non_local_info.CurrentUsage;
                }
            }

            adapters.push(info);
        }
    }

    adapters
}

pub fn collect_all_process_gpu_memory(
    pids: &[u32],
    adapters: &[GpuAdapterInfo],
) -> HashMap<u32, ProcessGpuMemory> {
    let mut map = HashMap::new();
    if adapters.is_empty() {
        return map;
    }

    let gdi = match Gdi32D3DKmt::load() {
        Some(g) => g,
        None => return map,
    };

    let query_fn = match gdi.query_statistics {
        Some(f) => f,
        None => return map,
    };

    for &pid in pids {
        if pid == 0 {
            continue;
        }

        unsafe {
            let handle_res = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
            let handle = match handle_res {
                Ok(h) if !h.is_invalid() => h,
                _ => continue,
            };

            let mut total_dedicated = 0u64;
            let mut total_shared = 0u64;

            for adapter in adapters {
                // Query Local (Dedicated VRAM) - SegmentGroup 0
                {
                    let mut query_data = D3DKMT_QUERYSTATISTICS {
                        r#type: D3DKMT_QUERYSTATISTICS_PROCESS_SEGMENT_GROUP,
                        adapter_luid: adapter.luid,
                        process_handle: handle,
                        query_result: [0; 320],
                        query_process_segment_group: 0, // Local / Dedicated
                        reserved_union: [0; 7],
                    };

                    let status = query_fn(&mut query_data as *mut _ as *mut c_void);
                    if status >= 0 {
                        let info = *(query_data.query_result.as_ptr()
                            as *const D3DKMT_QUERYSTATISTICS_PROCESS_SEGMENT_GROUP_INFORMATION);
                        let val = info.usage.max(info.requested);
                        total_dedicated = total_dedicated.saturating_add(val);
                    }
                }

                // Query NonLocal (Shared GPU Memory) - SegmentGroup 1
                {
                    let mut query_data = D3DKMT_QUERYSTATISTICS {
                        r#type: D3DKMT_QUERYSTATISTICS_PROCESS_SEGMENT_GROUP,
                        adapter_luid: adapter.luid,
                        process_handle: handle,
                        query_result: [0; 320],
                        query_process_segment_group: 1, // NonLocal / Shared
                        reserved_union: [0; 7],
                    };

                    let status = query_fn(&mut query_data as *mut _ as *mut c_void);
                    if status >= 0 {
                        let info = *(query_data.query_result.as_ptr()
                            as *const D3DKMT_QUERYSTATISTICS_PROCESS_SEGMENT_GROUP_INFORMATION);
                        let val = info.usage.max(info.requested);
                        total_shared = total_shared.saturating_add(val);
                    }
                }
            }

            let _ = CloseHandle(handle);

            if total_dedicated > 0 || total_shared > 0 {
                map.insert(
                    pid,
                    ProcessGpuMemory {
                        dedicated_bytes: total_dedicated,
                        shared_bytes: total_shared,
                    },
                );
            }
        }
    }

    map
}
