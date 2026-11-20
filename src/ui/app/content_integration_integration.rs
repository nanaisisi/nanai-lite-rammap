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
    let dedicated_metric = MemoryMetric::GpuDedicated;
    let shared_metric = MemoryMetric::GpuShared;

    let dedicated_view = match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => individual_treemap_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                dedicated_metric,
                SPLIT_CANVAS_WIDTH,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                dedicated_metric,
                SPLIT_CANVAS_WIDTH,
            ),
        },
        ViewMode::List => match app.group_mode {
            GroupMode::Individual => individual_list_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                dedicated_metric,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_list_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                dedicated_metric,
            ),
        },
    };

    let shared_view = match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => individual_treemap_with_metric(
                app,
                sender,
                filtered,
                shared_metric,
                SPLIT_CANVAS_WIDTH,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                app,
                sender,
                filtered,
                shared_metric,
                SPLIT_CANVAS_WIDTH,
            ),
        },
        ViewMode::List => match app.group_mode {
            GroupMode::Individual => {
                individual_list_with_metric(app, sender, filtered, shared_metric)
            }
            GroupMode::ByName | GroupMode::ByCategory => {
                grouped_list_with_metric(app, sender, filtered, shared_metric)
            }
        },
    };

    let dedicated_panel = StackPanel::new().spacing(4.0).children((
        TextBlock::new()
            .text("🎮 GPU Dedicated VRAM")
            .font_size(14.0)
            .font_weight(FontWeight::BOLD),
        dedicated_view,
    ));
    let shared_panel = StackPanel::new().spacing(4.0).children((
        TextBlock::new()
            .text("🎮 GPU Shared Memory")
            .font_size(14.0)
            .font_weight(FontWeight::BOLD),
        shared_view,
    ));

    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .children((dedicated_panel, shared_panel))
        .into()
}
