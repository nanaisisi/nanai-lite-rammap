use super::content_integration::{
    grouped_list, grouped_list_with_metric, individual_list, individual_list_with_metric,
};
use super::content_integration_core::{
    grouped_treemap, grouped_treemap_with_metric, individual_treemap,
    individual_treemap_with_metric,
};
use crate::memory::ProcessMemoryEntry;
use crate::ui::app_core::RammapApp;
use crate::ui::types::{
    GroupMode, MemoryMetric, MemoryTab, RammapMessage, SplitLayoutMode, ViewMode,
};
use windows_reactor::{
    FontWeight, Orientation, ScrollBarVisibility, ScrollViewer, StackPanel, TextBlock, View,
};

const SPLIT_CANVAS_WIDTH: f64 = 960.0;
const SPLIT_CANVAS_HEIGHT: f64 = 220.0;

pub(super) fn build<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
where
    S: Fn(RammapMessage) + Clone + 'static,
{
    match app.tab {
        MemoryTab::SystemRam | MemoryTab::VirtualMemory | MemoryTab::GpuVram => {
            build_single(app, sender, filtered)
        }
        MemoryTab::TotalMemory => build_total_memory(app, sender, filtered),
        MemoryTab::AllMemory => build_all_memory(app, sender, filtered),
        MemoryTab::Split => build_split(app, sender, filtered),
    }
}

fn build_single<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
where
    S: Fn(RammapMessage) + Clone + 'static,
{
    if app.tab == MemoryTab::GpuVram {
        return build_gpu_single(app, sender, filtered);
    }

    let total_capacity_bytes = if !app.show_free_space || !app.search_query.is_empty() {
        None
    } else if app.tab == MemoryTab::VirtualMemory {
        Some(app.summary.pagefile_total_bytes)
    } else if app.tab == MemoryTab::TotalMemory || app.use_requested {
        Some(app.summary.commit_limit_bytes)
    } else {
        Some(app.summary.total_phys_bytes)
    };

    match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => {
                individual_treemap(app, sender, filtered, total_capacity_bytes)
            }
            GroupMode::ByName | GroupMode::ByCategory => {
                grouped_treemap(app, sender, filtered, total_capacity_bytes)
            }
        },
        ViewMode::List => match app.group_mode {
            GroupMode::Individual => individual_list(app, sender, filtered),
            GroupMode::ByName | GroupMode::ByCategory => grouped_list(app, sender, filtered),
        },
    }
}

fn build_total_memory<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
where
    S: Fn(RammapMessage) + Clone + 'static,
{
    let phys_cap = if app.show_free_space && app.search_query.is_empty() {
        Some(app.summary.total_phys_bytes)
    } else {
        None
    };
    let pf_cap = if app.show_free_space && app.search_query.is_empty() {
        Some(app.summary.pagefile_total_bytes)
    } else {
        None
    };

    let (phys_h, pf_h) = if app.proportional_gpu {
        let total_h = SPLIT_CANVAS_HEIGHT * 2.0;
        let p = app.summary.total_phys_bytes as f64;
        let v = app.summary.pagefile_total_bytes as f64;
        let sum = (p + v).max(1.0);
        let p_h = (total_h * (p / sum)).clamp(8.0, total_h - 8.0);
        (p_h, (total_h - p_h).max(8.0))
    } else {
        (SPLIT_CANVAS_HEIGHT, SPLIT_CANVAS_HEIGHT)
    };

    let phys_metric = if app.use_requested {
        MemoryMetric::PrivateWs
    } else {
        MemoryMetric::WorkingSet
    };

    let phys_view = match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => individual_treemap_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                phys_metric,
                SPLIT_CANVAS_WIDTH,
                phys_h,
                phys_cap,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                phys_metric,
                SPLIT_CANVAS_WIDTH,
                phys_h,
                phys_cap,
            ),
        },
        ViewMode::List => match app.group_mode {
            GroupMode::Individual => {
                individual_list_with_metric(app, sender.clone(), filtered.clone(), phys_metric)
            }
            GroupMode::ByName | GroupMode::ByCategory => {
                grouped_list_with_metric(app, sender.clone(), filtered.clone(), phys_metric)
            }
        },
    };

    let pf_view = match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => individual_treemap_with_metric(
                app,
                sender,
                filtered,
                MemoryMetric::PagefileUsage,
                SPLIT_CANVAS_WIDTH,
                pf_h,
                pf_cap,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                app,
                sender,
                filtered,
                MemoryMetric::PagefileUsage,
                SPLIT_CANVAS_WIDTH,
                pf_h,
                pf_cap,
            ),
        },
        ViewMode::List => match app.group_mode {
            GroupMode::Individual => {
                individual_list_with_metric(app, sender, filtered, MemoryMetric::PagefileUsage)
            }
            GroupMode::ByName | GroupMode::ByCategory => {
                grouped_list_with_metric(app, sender, filtered, MemoryMetric::PagefileUsage)
            }
        },
    };

    StackPanel::new()
        .spacing(6.0)
        .children((
            TextBlock::new()
                .text(format!(
                    "💻 Physical RAM (Total: {} | Used: {})",
                    crate::memory::format_bytes(app.summary.total_phys_bytes),
                    crate::memory::format_bytes(app.summary.used_phys_bytes)
                ))
                .font_size(12.0),
            phys_view,
            TextBlock::new()
                .text(format!(
                    "💾 Virtual Memory / PageFile (Total: {} | Used: {})",
                    crate::memory::format_bytes(app.summary.pagefile_total_bytes),
                    crate::memory::format_bytes(app.summary.pagefile_in_use_bytes)
                ))
                .font_size(12.0),
            pf_view,
        ))
        .into()
}

fn build_all_memory<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
where
    S: Fn(RammapMessage) + Clone + 'static,
{
    let dgpus: Vec<(usize, &crate::gpu::GpuAdapterInfo)> = app
        .gpu_adapters
        .iter()
        .enumerate()
        .filter(|(_, g)| g.is_discrete)
        .collect();

    let total_cap: u64 = app
        .summary
        .total_phys_bytes
        .saturating_add(app.summary.pagefile_total_bytes)
        .saturating_add(dgpus.iter().map(|(_, g)| g.dedicated_vram_bytes).sum::<u64>())
        .max(1);

    let panel_count = 2 + dgpus.len();
    let total_h = SPLIT_CANVAS_HEIGHT * 2.0;
    let calc_h = |cap: u64| -> f64 {
        if app.proportional_gpu {
            (total_h * (cap as f64 / total_cap as f64)).max(24.0)
        } else {
            (total_h / panel_count as f64).max(80.0)
        }
    };

    let phys_h = calc_h(app.summary.total_phys_bytes);
    let pf_h = calc_h(app.summary.pagefile_total_bytes);

    let phys_cap = if app.show_free_space && app.search_query.is_empty() {
        Some(app.summary.total_phys_bytes)
    } else {
        None
    };
    let pf_cap = if app.show_free_space && app.search_query.is_empty() {
        Some(app.summary.pagefile_total_bytes)
    } else {
        None
    };

    let phys_metric = if app.use_requested {
        MemoryMetric::PrivateWs
    } else {
        MemoryMetric::WorkingSet
    };

    let phys_view = match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => individual_treemap_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                phys_metric,
                SPLIT_CANVAS_WIDTH,
                phys_h,
                phys_cap,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                phys_metric,
                SPLIT_CANVAS_WIDTH,
                phys_h,
                phys_cap,
            ),
        },
        ViewMode::List => match app.group_mode {
            GroupMode::Individual => {
                individual_list_with_metric(app, sender.clone(), filtered.clone(), phys_metric)
            }
            GroupMode::ByName | GroupMode::ByCategory => {
                grouped_list_with_metric(app, sender.clone(), filtered.clone(), phys_metric)
            }
        },
    };

    let pf_view = match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => individual_treemap_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                MemoryMetric::PagefileUsage,
                SPLIT_CANVAS_WIDTH,
                pf_h,
                pf_cap,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                MemoryMetric::PagefileUsage,
                SPLIT_CANVAS_WIDTH,
                pf_h,
                pf_cap,
            ),
        },
        ViewMode::List => match app.group_mode {
            GroupMode::Individual => individual_list_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                MemoryMetric::PagefileUsage,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_list_with_metric(
                app,
                sender.clone(),
                filtered.clone(),
                MemoryMetric::PagefileUsage,
            ),
        },
    };

    let mut children: Vec<View> = vec![
        TextBlock::new()
            .text(format!(
                "💻 Physical RAM (Total: {} | Used: {})",
                crate::memory::format_bytes(app.summary.total_phys_bytes),
                crate::memory::format_bytes(app.summary.used_phys_bytes)
            ))
            .font_size(12.0)
            .into(),
        phys_view,
        TextBlock::new()
            .text(format!(
                "💾 Virtual Memory / PageFile (Total: {} | Used: {})",
                crate::memory::format_bytes(app.summary.pagefile_total_bytes),
                crate::memory::format_bytes(app.summary.pagefile_in_use_bytes)
            ))
            .font_size(12.0)
            .into(),
        pf_view,
    ];

    for (idx, adapter) in dgpus {
        let d_h = calc_h(adapter.dedicated_vram_bytes);
        let d_cap = if app.show_free_space && app.search_query.is_empty() {
            Some(adapter.dedicated_vram_bytes)
        } else {
            None
        };
        let adapter_processes: Vec<ProcessMemoryEntry> = filtered
            .iter()
            .cloned()
            .map(|mut p| {
                let usage = p.gpu_per_adapter.get(idx).copied().unwrap_or_default();
                p.gpu_dedicated_bytes = if app.use_requested {
                    usage.dedicated_requested
                } else {
                    usage.dedicated_usage
                };
                p
            })
            .collect();

        let d_view = match app.view_mode {
            ViewMode::Treemap => match app.group_mode {
                GroupMode::Individual => individual_treemap_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes,
                    MemoryMetric::GpuDedicated,
                    SPLIT_CANVAS_WIDTH,
                    d_h,
                    d_cap,
                ),
                GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes,
                    MemoryMetric::GpuDedicated,
                    SPLIT_CANVAS_WIDTH,
                    d_h,
                    d_cap,
                ),
            },
            ViewMode::List => match app.group_mode {
                GroupMode::Individual => individual_list_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes,
                    MemoryMetric::GpuDedicated,
                ),
                GroupMode::ByName | GroupMode::ByCategory => grouped_list_with_metric(
                    app,
                    sender.clone(),
                    adapter_processes,
                    MemoryMetric::GpuDedicated,
                ),
            },
        };

        children.push(
            TextBlock::new()
                .text(format!(
                    "🎮 GPU {} [dGPU]: {} (Dedicated VRAM: {})",
                    idx,
                    adapter.name,
                    crate::memory::format_bytes(adapter.dedicated_vram_bytes)
                ))
                .font_size(12.0)
                .into(),
        );
        children.push(d_view);
    }

    StackPanel::new().spacing(6.0).children(children).into()
}

fn build_gpu_single<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
where
    S: Fn(RammapMessage) + Clone + 'static,
{
    let idx = app.selected_gpu_index;
    let Some(adapter) = app.gpu_adapters.get(idx) else {
        return TextBlock::new()
            .text("No DirectX 11/12 GPU detected")
            .font_size(14.0)
            .into();
    };
    let max_total_bytes = app
        .gpu_adapters
        .iter()
        .map(|g| g.dedicated_vram_bytes + g.shared_ram_bytes)
        .max()
        .unwrap_or(1)
        .max(1);
    build_gpu_column(
        app,
        sender,
        &filtered,
        idx,
        adapter,
        SPLIT_CANVAS_WIDTH,
        max_total_bytes,
    )
}

fn build_split<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
where
    S: Fn(RammapMessage) + Clone + 'static,
{
    let adapter_count = app.gpu_adapters.len().max(1);
    let panel_width = match app.split_layout_mode {
        SplitLayoutMode::Fit => {
            ((SPLIT_CANVAS_WIDTH - 12.0 * (adapter_count - 1) as f64) / adapter_count as f64)
                .max(160.0)
        }
        SplitLayoutMode::Scroll => {
            if adapter_count == 1 {
                SPLIT_CANVAS_WIDTH
            } else {
                494.0
            }
        }
    };

    let mut gpu_columns: Vec<View> = Vec::with_capacity(adapter_count);

    let max_total_bytes = app
        .gpu_adapters
        .iter()
        .map(|g| g.dedicated_vram_bytes + g.shared_ram_bytes)
        .max()
        .unwrap_or(1)
        .max(1);

    for (idx, adapter) in app.gpu_adapters.iter().enumerate() {
        gpu_columns.push(build_gpu_column(
            app,
            sender.clone(),
            &filtered,
            idx,
            adapter,
            panel_width,
            max_total_bytes,
        ));
    }

    if gpu_columns.is_empty() {
        return TextBlock::new()
            .text("No DirectX 11/12 GPU detected")
            .font_size(14.0)
            .into();
    }

    let row = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .children(gpu_columns);

    match app.split_layout_mode {
        SplitLayoutMode::Fit => row.into(),
        SplitLayoutMode::Scroll => ScrollViewer::new()
            .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
            .vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled)
            .width(SPLIT_CANVAS_WIDTH)
            .content(row)
            .into(),
    }
}

fn build_gpu_column<S>(
    app: &RammapApp,
    sender: S,
    filtered: &[ProcessMemoryEntry],
    idx: usize,
    adapter: &crate::gpu::GpuAdapterInfo,
    panel_width: f64,
    max_total_bytes: u64,
) -> View
where
    S: Fn(RammapMessage) + Clone + 'static,
{
    let adapter_processes: Vec<ProcessMemoryEntry> = filtered
        .iter()
        .cloned()
        .map(|mut p| {
            let usage = p.gpu_per_adapter.get(idx).copied().unwrap_or_default();
            if app.use_requested {
                p.gpu_dedicated_bytes = usage.dedicated_requested;
                p.gpu_shared_bytes = usage.shared_requested;
            } else {
                p.gpu_dedicated_bytes = usage.dedicated_usage;
                p.gpu_shared_bytes = usage.shared_usage;
            }
            p
        })
        .collect();

    let ded_used: u64 = adapter_processes.iter().map(|p| p.gpu_dedicated_bytes).sum();
    let ded_free = adapter.dedicated_vram_bytes.saturating_sub(ded_used);
    let shd_used: u64 = adapter_processes.iter().map(|p| p.gpu_shared_bytes).sum();
    let shd_free = adapter.shared_ram_bytes.saturating_sub(shd_used);

    let dedicated_cap = if app.show_free_space && app.search_query.is_empty() {
        Some(adapter.dedicated_vram_bytes)
    } else {
        None
    };
    let shared_cap = if app.show_free_space && app.search_query.is_empty() {
        Some(adapter.shared_ram_bytes)
    } else {
        None
    };

    let (dedicated_width, dedicated_height, shared_width, shared_height) = if app.proportional_gpu
    {
        let ded = adapter.dedicated_vram_bytes as f64;
        let shd = adapter.shared_ram_bytes as f64;
        let gpu_sum = (ded + shd).max(1.0);
        let scale = (gpu_sum / (max_total_bytes as f64)).clamp(0.05, 1.0);
        let total_h = (SPLIT_CANVAS_HEIGHT * 2.0 * scale).max(16.0);
        let ideal_d_h = total_h * (ded / gpu_sum);
        let ideal_s_h = total_h * (shd / gpu_sum);
        let d_h = ideal_d_h.clamp(8.0, total_h - 8.0);
        let s_h = (total_h - d_h).max(8.0);
        let d_w = if ideal_d_h < d_h {
            (panel_width * (ideal_d_h / d_h)).max(8.0)
        } else {
            panel_width
        };
        let s_w = if ideal_s_h < s_h {
            (panel_width * (ideal_s_h / s_h)).max(8.0)
        } else {
            panel_width
        };
        (d_w, d_h, s_w, s_h)
    } else {
        (
            panel_width,
            SPLIT_CANVAS_HEIGHT,
            panel_width,
            SPLIT_CANVAS_HEIGHT,
        )
    };

    let dedicated_view = match app.view_mode {
        ViewMode::Treemap => match app.group_mode {
            GroupMode::Individual => individual_treemap_with_metric(
                app,
                sender.clone(),
                adapter_processes.clone(),
                MemoryMetric::GpuDedicated,
                dedicated_width,
                dedicated_height,
                dedicated_cap,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                app,
                sender.clone(),
                adapter_processes.clone(),
                MemoryMetric::GpuDedicated,
                dedicated_width,
                dedicated_height,
                dedicated_cap,
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
                shared_width,
                shared_height,
                shared_cap,
            ),
            GroupMode::ByName | GroupMode::ByCategory => grouped_treemap_with_metric(
                app,
                sender.clone(),
                adapter_processes.clone(),
                MemoryMetric::GpuShared,
                shared_width,
                shared_height,
                shared_cap,
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
                sender,
                adapter_processes,
                MemoryMetric::GpuShared,
            ),
        },
    };

    let kind = if adapter.is_discrete { "dGPU" } else { "iGPU" };
    StackPanel::new()
        .spacing(6.0)
        .children((
            TextBlock::new()
                .text(format!("🎮 GPU {} [{}]: {}", idx, kind, adapter.name))
                .font_size(14.0)
                .font_weight(FontWeight::BOLD),
            TextBlock::new()
                .text(format!(
                    "Dedicated VRAM (Total: {} | Free: {})",
                    crate::memory::format_bytes(adapter.dedicated_vram_bytes),
                    crate::memory::format_bytes(ded_free)
                ))
                .font_size(12.0),
            dedicated_view,
            TextBlock::new()
                .text(format!(
                    "Shared GPU Memory (Total: {} | Free: {})",
                    crate::memory::format_bytes(adapter.shared_ram_bytes),
                    crate::memory::format_bytes(shd_free)
                ))
                .font_size(12.0),
            shared_view,
        ))
        .into()
}
