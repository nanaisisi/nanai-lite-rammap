use super::content_integration::{
    grouped_list, grouped_list_with_metric, individual_list, individual_list_with_metric,
};
use super::content_integration_core::{
    grouped_treemap, grouped_treemap_with_metric, individual_treemap,
    individual_treemap_with_metric,
};
use crate::memory::ProcessMemoryEntry;
use crate::ui::app_core::RammapApp;
use crate::ui::types::{GroupMode, MemoryMetric, MemoryTab, RammapMessage, ViewMode};
use windows_reactor::{FontWeight, Orientation, StackPanel, TextBlock, View};

const SPLIT_CANVAS_WIDTH: f64 = 1000.0;
const SPLIT_CANVAS_HEIGHT: f64 = 260.0;

pub(super) fn build<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
where
    S: Fn(RammapMessage) + Clone + 'static,
{
    match app.tab {
        MemoryTab::SystemRam | MemoryTab::GpuVram => build_single(app, sender, filtered),
        MemoryTab::Split => build_split(app, sender, filtered),
    }
}

fn build_single<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
where
    S: Fn(RammapMessage) + Clone + 'static,
{
    match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => individual_treemap(app, sender, filtered),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap(app, sender, filtered),
        },
        ViewMode::List => match app.group_mode {
            GroupMode::Individual => individual_list(app, sender, filtered),
            GroupMode::ByName | GroupMode::ByCategory => grouped_list(app, sender, filtered),
        },
    }
}

fn build_split<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
where
    S: Fn(RammapMessage) + Clone + 'static,
{
    let adapter_count = app.gpu_adapters.len().max(1);
    let panel_width =
        ((SPLIT_CANVAS_WIDTH - 12.0 * (adapter_count - 1) as f64) / adapter_count as f64).max(300.0);

    let mut gpu_columns: Vec<View> = Vec::with_capacity(adapter_count);

    for (idx, adapter) in app.gpu_adapters.iter().enumerate() {
        let adapter_processes: Vec<ProcessMemoryEntry> = filtered
            .iter()
            .cloned()
            .map(|mut p| {
                let (ded, shd) = p.gpu_per_adapter.get(idx).copied().unwrap_or((0, 0));
                p.gpu_dedicated_bytes = ded;
                p.gpu_shared_bytes = shd;
                p
            })
            .collect();

        let dedicated_view = match app.view_mode {
            ViewMode::Treemap => match app.group_mode {
                GroupMode::Individual => individual_treemap_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes.clone(),
                    MemoryMetric::GpuDedicated,
                    panel_width,
                    SPLIT_CANVAS_HEIGHT,
                ),
                GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes.clone(),
                    MemoryMetric::GpuDedicated,
                    panel_width,
                    SPLIT_CANVAS_HEIGHT,
                ),
            },
            ViewMode::List => match app.group_mode {
                GroupMode::Individual => individual_list_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes.clone(),
                    MemoryMetric::GpuDedicated,
                ),
                GroupMode::ByName | GroupMode::ByCategory => grouped_list_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes.clone(),
                    MemoryMetric::GpuDedicated,
                ),
            },
        };

        let shared_view = match app.view_mode {
            ViewMode::Treemap => match app.group_mode {
                GroupMode::Individual => individual_treemap_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes.clone(),
                    MemoryMetric::GpuShared,
                    panel_width,
                    SPLIT_CANVAS_HEIGHT,
                ),
                GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes.clone(),
                    MemoryMetric::GpuShared,
                    panel_width,
                    SPLIT_CANVAS_HEIGHT,
                ),
            },
            ViewMode::List => match app.group_mode {
                GroupMode::Individual => individual_list_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes.clone(),
                    MemoryMetric::GpuShared,
                ),
                GroupMode::ByName | GroupMode::ByCategory => grouped_list_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes,
                    MemoryMetric::GpuShared,
                ),
            },
        };

        let kind = if adapter.is_discrete { "dGPU" } else { "iGPU" };
        let column = StackPanel::new()
            .spacing(6.0)
            .children((
                TextBlock::new()
                    .text(format!("🎮 GPU {} [{}]: {}", idx, kind, adapter.name))
                    .font_size(14.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new().text("Dedicated VRAM").font_size(12.0),
                dedicated_view,
                TextBlock::new().text("Shared GPU Memory").font_size(12.0),
                shared_view,
            ))
            .into();
        gpu_columns.push(column);
    }

    if gpu_columns.is_empty() {
        return TextBlock::new()
            .text("No DirectX 11/12 GPU detected")
            .font_size(14.0)
            .into();
    }

    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .children(gpu_columns)
        .into()
}
