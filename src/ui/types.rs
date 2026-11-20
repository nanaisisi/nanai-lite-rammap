use crate::memory::ProcessMemoryEntry;

#[derive(Clone, PartialEq, Default)]
pub struct AppInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTab {
    SystemRam,
    VirtualMemory,
    GpuVram,
    Split,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Treemap,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitLayoutMode {
    Fit,
    Scroll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMode {
    Individual,
    ByName,
    ByCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMetric {
    WorkingSet,
    PrivateWs,
    GpuDedicated,
    GpuShared,
}

#[derive(Clone)]
pub enum RammapMessage {
    Refresh,
    SetTab(MemoryTab),
    SearchChanged(String),
    SetViewMode(ViewMode),
    SetGroupMode(GroupMode),
    ToggleSplitLayoutMode,
    ToggleFreeSpace,
    ToggleProportionalGpu,
    CycleGpuAdapter,
    ToggleMetric,
    SelectProcess(Option<ProcessMemoryEntry>, Option<String>),
}
