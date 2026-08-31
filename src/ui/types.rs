use crate::memory::ProcessMemoryEntry;

#[derive(Clone, PartialEq, Default)]
pub struct AppInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTab {
    SystemRam,
    GpuVram,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Treemap,
    List,
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

pub enum RammapMessage {
    Refresh,
    SetTab(MemoryTab),
    SearchChanged(String),
    SetViewMode(ViewMode),
    SetGroupMode(GroupMode),
    ToggleMetric,
    SelectProcess(Option<ProcessMemoryEntry>, Option<String>),
}
