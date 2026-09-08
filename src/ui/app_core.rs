use crate::gpu::{GpuAdapterInfo, collect_all_process_gpu_memory, collect_gpu_adapters};
use crate::memory::{
    ProcessMemoryEntry, SystemMemorySummary, collect_process_memory, collect_system_summary,
};
use crate::ui::types::{AppInput, GroupMode, MemoryMetric, MemoryTab, RammapMessage, ViewMode};

pub struct RammapApp {
    pub(super) tab: MemoryTab,
    pub(super) summary: SystemMemorySummary,
    pub(super) gpu_adapters: Vec<GpuAdapterInfo>,
    pub(super) processes: Vec<ProcessMemoryEntry>,
    pub(super) search_query: String,
    pub(super) view_mode: ViewMode,
    pub(super) group_mode: GroupMode,
    pub(super) metric: MemoryMetric,
    pub(super) selected_process: Option<ProcessMemoryEntry>,
    pub(super) selected_group_title: Option<String>,
}

fn fetch_all_processes(gpu_adapters: &[GpuAdapterInfo]) -> Vec<ProcessMemoryEntry> {
    let mut processes = collect_process_memory();
    let pids: Vec<u32> = processes.iter().map(|p| p.pid).collect();
    let gpu_map = collect_all_process_gpu_memory(&pids, gpu_adapters);

    for p in &mut processes {
        if let Some(gpu_info) = gpu_map.get(&p.pid) {
            p.gpu_dedicated_bytes = gpu_info.dedicated_bytes;
            p.gpu_shared_bytes = gpu_info.shared_bytes;
        }
    }
    processes
}

impl RammapApp {
    pub(super) fn new(_input: &AppInput) -> Self {
        let summary = collect_system_summary();
        let gpu_adapters = collect_gpu_adapters();
        let processes = fetch_all_processes(&gpu_adapters);
        Self {
            tab: MemoryTab::SystemRam,
            summary,
            gpu_adapters,
            processes,
            search_query: String::new(),
            view_mode: ViewMode::Treemap,
            group_mode: GroupMode::ByName,
            metric: MemoryMetric::WorkingSet,
            selected_process: None,
            selected_group_title: None,
        }
    }

    pub(super) fn update_message(&mut self, message: RammapMessage) {
        match message {
            RammapMessage::Refresh => {
                self.summary = collect_system_summary();
                self.gpu_adapters = collect_gpu_adapters();
                self.processes = fetch_all_processes(&self.gpu_adapters);
                if let Some(ref sel) = self.selected_process {
                    let pid = sel.pid;
                    self.selected_process = self.processes.iter().find(|p| p.pid == pid).cloned();
                }
            }
            RammapMessage::SetTab(tab) => {
                self.tab = tab;
                if tab == MemoryTab::GpuVram
                    && self.metric != MemoryMetric::GpuDedicated
                    && self.metric != MemoryMetric::GpuShared
                {
                    self.metric = MemoryMetric::GpuDedicated;
                } else if tab == MemoryTab::SystemRam
                    && (self.metric == MemoryMetric::GpuDedicated
                        || self.metric == MemoryMetric::GpuShared)
                {
                    self.metric = MemoryMetric::WorkingSet;
                }
            }
            RammapMessage::SearchChanged(query) => self.search_query = query,
            RammapMessage::SetViewMode(mode) => self.view_mode = mode,
            RammapMessage::SetGroupMode(mode) => self.group_mode = mode,
            RammapMessage::ToggleMetric => {
                self.metric = match self.tab {
                    MemoryTab::SystemRam => match self.metric {
                        MemoryMetric::WorkingSet => MemoryMetric::PrivateWs,
                        _ => MemoryMetric::WorkingSet,
                    },
                    MemoryTab::GpuVram => match self.metric {
                        MemoryMetric::GpuDedicated => MemoryMetric::GpuShared,
                        _ => MemoryMetric::GpuDedicated,
                    },
                };
            }
            RammapMessage::SelectProcess(process, group_title) => {
                self.selected_process = process;
                self.selected_group_title = group_title;
            }
        }
    }
}
