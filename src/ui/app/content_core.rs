use crate::category::ProcessGroup;
use crate::memory::{format_bytes, ProcessMemoryEntry};
use crate::ui::types::{MemoryMetric, MemoryTab};

pub(super) fn metric_value(p: &ProcessMemoryEntry, metric: MemoryMetric) -> f64 {
    match metric {
        MemoryMetric::WorkingSet => p.working_set_bytes as f64,
        MemoryMetric::PrivateWs => p.private_bytes as f64,
        MemoryMetric::GpuDedicated => p.gpu_dedicated_bytes as f64,
        MemoryMetric::GpuShared => p.gpu_shared_bytes as f64,
    }
}

pub(super) fn group_metric_value(
    group: &ProcessGroup<ProcessMemoryEntry>,
    metric: MemoryMetric,
) -> f64 {
    match metric {
        MemoryMetric::WorkingSet => group.total_working_set as f64,
        MemoryMetric::PrivateWs => group.total_private as f64,
        MemoryMetric::GpuDedicated => group.total_gpu_dedicated as f64,
        MemoryMetric::GpuShared => group.total_gpu_shared as f64,
    }
}

pub(super) fn metric_bytes(p: &ProcessMemoryEntry, metric: MemoryMetric) -> u64 {
    metric_value(p, metric) as u64
}

pub(super) fn group_metric_bytes(
    group: &ProcessGroup<ProcessMemoryEntry>,
    metric: MemoryMetric,
) -> u64 {
    group_metric_value(group, metric) as u64
}

pub(super) fn group_info(tab: MemoryTab, group: &ProcessGroup<ProcessMemoryEntry>) -> String {
    match tab {
        MemoryTab::SystemRam => format!(
            "{} | Total WS: {} | Total Private: {} | Count: {}",
            group.title,
            format_bytes(group.total_working_set),
            format_bytes(group.total_private),
            group.items.len()
        ),
        MemoryTab::GpuVram => format!(
            "{} | Total Dedicated VRAM: {} | Total Shared: {} | Count: {}",
            group.title,
            format_bytes(group.total_gpu_dedicated),
            format_bytes(group.total_gpu_shared),
            group.items.len()
        ),
    }
}

pub(super) fn sort_processes(processes: &mut [ProcessMemoryEntry], metric: MemoryMetric) {
    processes.sort_by_key(|process| std::cmp::Reverse(metric_bytes(process, metric)));
}

pub(super) fn sort_groups(groups: &mut [ProcessGroup<ProcessMemoryEntry>], metric: MemoryMetric) {
    groups.sort_by_key(|group| std::cmp::Reverse(group_metric_bytes(group, metric)));
}
