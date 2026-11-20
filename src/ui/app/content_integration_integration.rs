use super::content_integration::{grouped_list, individual_list};
use super::content_integration_core::{
    grouped_treemap, grouped_treemap_with_metric, individual_treemap,
    individual_treemap_with_metric,
};
use crate::memory::ProcessMemoryEntry;
use crate::ui::app_core::RammapApp;
use crate::ui::types::{GroupMode, MemoryMetric, MemoryTab, RammapMessage, ViewMode};
use windows_reactor::{FontWeight, Orientation, StackPanel, TextBlock, View};

const SPLIT_CANVAS_WIDTH: f64 = 494.0;

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
    let ram_metric = match app.metric {
        MemoryMetric::PrivateWs => MemoryMetric::PrivateWs,
        _ => MemoryMetric::WorkingSet,
    };
    let gpu_metric = match app.metric {
        MemoryMetric::GpuShared => MemoryMetric::GpuShared,
        _ => MemoryMetric::GpuDedicated,
    };

    let ram_view = match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => individual_treemap_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                ram_metric,
                SPLIT_CANVAS_WIDTH,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                ram_metric,
                SPLIT_CANVAS_WIDTH,
            ),
        },
        ViewMode::List => match app.group_mode {
            GroupMode::Individual => individual_list(app, sender.clone(), filtered.clone()),
            GroupMode::ByName | GroupMode::ByCategory => {
                grouped_list(app, sender.clone(), filtered.clone())
            }
        },
    };

    let gpu_view = match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => individual_treemap_with_metric(
                app,
                sender,
                filtered,
                gpu_metric,
                SPLIT_CANVAS_WIDTH,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                app,
                sender,
                filtered,
                gpu_metric,
                SPLIT_CANVAS_WIDTH,
            ),
        },
        ViewMode::List => match app.group_mode {
            GroupMode::Individual => individual_list(app, sender, filtered),
            GroupMode::ByName | GroupMode::ByCategory => grouped_list(app, sender, filtered),
        },
    };

    let ram_panel = StackPanel::new().spacing(4.0).children((
        TextBlock::new()
            .text("💻 System RAM")
            .font_size(14.0)
            .font_weight(FontWeight::BOLD),
        ram_view,
    ));
    let gpu_panel = StackPanel::new().spacing(4.0).children((
        TextBlock::new()
            .text("🎮 GPU VRAM")
            .font_size(14.0)
            .font_weight(FontWeight::BOLD),
        gpu_view,
    ));

    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .children((ram_panel, gpu_panel))
        .into()
}
