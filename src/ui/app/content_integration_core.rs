use super::content_core::{
    group_info, group_metric_bytes, group_metric_value, metric_bytes, metric_value,
};
use crate::category::{classify_process, group_by_category, group_by_name};
use crate::memory::{ProcessMemoryEntry, format_bytes};
use crate::treemap::{Rect, TreemapItem, layout_treemap};
use crate::ui::app_core::RammapApp;
use crate::ui::theme::{color_for_category, wrap_canvas};
use crate::ui::types::{GroupMode, MemoryMetric, MemoryTab, RammapMessage};
use windows_reactor::*;

const CANVAS_WIDTH: f64 = 960.0;
const CANVAS_HEIGHT: f64 = 500.0;

fn bounds(width: f64, height: f64) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
    }
}

#[derive(Clone)]
enum TreemapEntry<T> {
    Item(T),
    System {
        label: &'static str,
        bytes: u64,
        color: Color,
    },
    Free(u64),
}

fn push_system_and_free<T>(
    items: &mut Vec<TreemapItem<TreemapEntry<T>>>,
    app: &RammapApp,
    metric: MemoryMetric,
    used_bytes: u64,
    total_capacity_bytes: Option<u64>,
) {
    let mut extra_used = 0u64;
    if app.tab == MemoryTab::SystemRam && app.search_query.is_empty() {
        let sys_blocks = [
            (
                "Non-Paged Pool",
                app.summary.nonpaged_pool_bytes,
                Color::argb(255, 150, 75, 110),
            ),
            (
                "System Cache",
                app.summary.system_cache_bytes,
                Color::argb(255, 85, 140, 100),
            ),
            (
                "Modified",
                app.summary.modified_bytes,
                Color::argb(255, 160, 120, 60),
            ),
        ];
        for (label, bytes, color) in sys_blocks {
            if bytes > 0 {
                extra_used = extra_used.saturating_add(bytes);
                items.push(TreemapItem {
                    value: bytes as f64,
                    data: TreemapEntry::System {
                        label,
                        bytes,
                        color,
                    },
                });
            }
        }
    } else if app.tab == MemoryTab::TotalMemory && app.search_query.is_empty() {
        let bytes = app.summary.paged_pool_bytes;
        if bytes > 0 {
            extra_used = extra_used.saturating_add(bytes);
            items.push(TreemapItem {
                value: bytes as f64,
                data: TreemapEntry::System {
                    label: "Paged Pool",
                    bytes,
                    color: Color::argb(255, 110, 80, 150),
                },
            });
        }
    } else if (app.tab == MemoryTab::VirtualMemory || metric == MemoryMetric::PagefileUsage)
        && app.search_query.is_empty()
    {
        items.clear();
        let pf_used = app.summary.pagefile_in_use_bytes;
        if pf_used > 0 {
            extra_used = pf_used;
            items.push(TreemapItem {
                value: pf_used as f64,
                data: TreemapEntry::System {
                    label: "PageFile Used",
                    bytes: pf_used,
                    color: Color::argb(255, 80, 130, 180),
                },
            });
        }
        let pf_free = total_capacity_bytes
            .map(|cap| cap.saturating_sub(extra_used))
            .unwrap_or(0);
        if pf_free > 0 {
            items.push(TreemapItem {
                value: pf_free as f64,
                data: TreemapEntry::Free(pf_free),
            });
        }
        return;
    }
    let free_bytes = total_capacity_bytes
        .map(|cap| cap.saturating_sub(used_bytes.saturating_add(extra_used)))
        .unwrap_or(0);
    if free_bytes > 0 {
        items.push(TreemapItem {
            value: free_bytes as f64,
            data: TreemapEntry::Free(free_bytes),
        });
    }
}

pub(super) fn individual_treemap<S: Fn(RammapMessage) + Clone + 'static>(
    app: &RammapApp,
    sender: S,
    processes: Vec<ProcessMemoryEntry>,
    total_capacity_bytes: Option<u64>,
) -> View {
    individual_treemap_with_metric(
        app,
        sender,
        processes,
        app.metric,
        CANVAS_WIDTH,
        CANVAS_HEIGHT,
        total_capacity_bytes,
    )
}

pub(super) fn individual_treemap_with_metric<S: Fn(RammapMessage) + Clone + 'static>(
    app: &RammapApp,
    sender: S,
    processes: Vec<ProcessMemoryEntry>,
    metric: MemoryMetric,
    canvas_width: f64,
    canvas_height: f64,
    total_capacity_bytes: Option<u64>,
) -> View {
    let used_bytes: u64 = processes.iter().map(|p| metric_bytes(p, metric)).sum();
    let mut items = processes
        .into_iter()
        .map(|p| TreemapItem {
            value: metric_value(&p, metric),
            data: TreemapEntry::Item(p),
        })
        .collect::<Vec<_>>();
    push_system_and_free(&mut items, app, metric, used_bytes, total_capacity_bytes);
    let mut elements = Vec::new();

    for node in layout_treemap(&items, bounds(canvas_width, canvas_height)) {
        let p = match node.data {
            TreemapEntry::Item(p) => p,
            TreemapEntry::System {
                label: name,
                bytes,
                color,
            } => {
                let label = if node.rect.width > 55.0 && node.rect.height > 26.0 {
                    format!("{}\n{}", name, format_bytes(bytes))
                } else {
                    String::new()
                };
                let click_sender = sender.clone();
                let block = Border::new()
                    .width(node.rect.width.max(2.0))
                    .height(node.rect.height.max(2.0))
                    .background(Brush::Solid(color))
                    .border_brush(Brush::Solid(Color::argb(200, 20, 20, 20)))
                    .border_thickness(Thickness::uniform(1.0))
                    .corner_radius(CornerRadius::uniform(3.0))
                    .canvas_left(node.rect.x)
                    .canvas_top(node.rect.y)
                    .on_pointer_pressed(move |_| {
                        click_sender(RammapMessage::SelectProcess(None, None));
                    })
                    .content(
                        TextBlock::new()
                            .text(label)
                            .font_size(11.0)
                            .margin(Thickness::xy(3.0, 2.0)),
                    );
                elements.push(block.into());
                continue;
            }
            TreemapEntry::Free(free_bytes) => {
                let label = if node.rect.width > 55.0 && node.rect.height > 26.0 {
                    format!("Free\n{}", format_bytes(free_bytes))
                } else {
                    String::new()
                };
                let click_sender = sender.clone();
                let block = Border::new()
                    .width(node.rect.width.max(2.0))
                    .height(node.rect.height.max(2.0))
                    .background(Brush::Solid(Color::argb(255, 45, 45, 50)))
                    .border_brush(Brush::Solid(Color::argb(200, 30, 30, 34)))
                    .border_thickness(Thickness::uniform(1.0))
                    .corner_radius(CornerRadius::uniform(3.0))
                    .canvas_left(node.rect.x)
                    .canvas_top(node.rect.y)
                    .on_pointer_pressed(move |_| {
                        click_sender(RammapMessage::SelectProcess(None, None));
                    })
                    .content(
                        TextBlock::new()
                            .text(label)
                            .font_size(11.0)
                            .margin(Thickness::xy(3.0, 2.0)),
                    );
                elements.push(block.into());
                continue;
            }
        };
        let pid = p.pid;
        let label_value = format_bytes(metric_bytes(&p, metric));
        let category = classify_process(&p.name);
        let selected = app.selected_process.as_ref().is_some_and(|s| s.pid == pid);
        let background = if selected {
            Color::argb(255, 240, 160, 40)
        } else {
            color_for_category(category, pid)
        };
        let border = if selected {
            Color::argb(255, 255, 255, 255)
        } else {
            Color::argb(180, 20, 20, 20)
        };
        let label = if node.rect.width > 55.0 && node.rect.height > 26.0 {
            if node.rect.height > 42.0 && node.rect.width > 70.0 {
                format!("{}\n{}", p.name, label_value)
            } else {
                format!("{} ({})", p.name, label_value)
            }
        } else if node.rect.width > 30.0 && node.rect.height > 18.0 {
            p.name.clone()
        } else {
            String::new()
        };
        let selected_process = p.clone();
        let click_sender = sender.clone();
        let block = Border::new()
            .width(node.rect.width.max(2.0))
            .height(node.rect.height.max(2.0))
            .background(Brush::Solid(background))
            .border_brush(Brush::Solid(border))
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(CornerRadius::uniform(2.0))
            .canvas_left(node.rect.x)
            .canvas_top(node.rect.y)
            .on_pointer_pressed(move |_| {
                click_sender(RammapMessage::SelectProcess(
                    Some(selected_process.clone()),
                    None,
                ));
            })
            .content(
                TextBlock::new()
                    .text(label)
                    .font_size(if node.rect.height > 35.0 { 11.0 } else { 9.5 })
                    .margin(Thickness::xy(3.0, 2.0)),
            );
        elements.push(block.into());
    }
    wrap_canvas(
        Canvas::new()
            .width(canvas_width)
            .height(canvas_height)
            .children(elements)
            .into(),
    )
}

pub(super) fn grouped_treemap<S: Fn(RammapMessage) + Clone + 'static>(
    app: &RammapApp,
    sender: S,
    processes: Vec<ProcessMemoryEntry>,
    total_capacity_bytes: Option<u64>,
) -> View {
    grouped_treemap_with_metric(
        app,
        sender,
        processes,
        app.metric,
        CANVAS_WIDTH,
        CANVAS_HEIGHT,
        total_capacity_bytes,
    )
}

pub(super) fn grouped_treemap_with_metric<S: Fn(RammapMessage) + Clone + 'static>(
    app: &RammapApp,
    sender: S,
    processes: Vec<ProcessMemoryEntry>,
    metric: MemoryMetric,
    canvas_width: f64,
    canvas_height: f64,
    total_capacity_bytes: Option<u64>,
) -> View {
    let groups = if app.group_mode == GroupMode::ByName {
        group_by_name(&processes)
    } else {
        group_by_category(&processes)
    };
    let used_bytes: u64 = groups.iter().map(|g| group_metric_bytes(g, metric)).sum();
    let mut items = groups
        .into_iter()
        .map(|g| TreemapItem {
            value: group_metric_value(&g, metric),
            data: TreemapEntry::Item(g),
        })
        .collect::<Vec<_>>();
    push_system_and_free(&mut items, app, metric, used_bytes, total_capacity_bytes);
    let mut elements = Vec::new();

    for (index, node) in layout_treemap(&items, bounds(canvas_width, canvas_height))
        .into_iter()
        .enumerate()
    {
        let group = match node.data {
            TreemapEntry::Item(g) => g,
            TreemapEntry::System {
                label: name,
                bytes,
                color,
            } => {
                let label = if node.rect.width > 60.0 && node.rect.height > 28.0 {
                    format!("{}\n{}", name, format_bytes(bytes))
                } else {
                    String::new()
                };
                let click_sender = sender.clone();
                let block = Border::new()
                    .width(node.rect.width.max(2.0))
                    .height(node.rect.height.max(2.0))
                    .background(Brush::Solid(color))
                    .border_brush(Brush::Solid(Color::argb(200, 20, 20, 20)))
                    .border_thickness(Thickness::uniform(1.0))
                    .corner_radius(CornerRadius::uniform(3.0))
                    .canvas_left(node.rect.x)
                    .canvas_top(node.rect.y)
                    .on_pointer_pressed(move |_| {
                        click_sender(RammapMessage::SelectProcess(None, None));
                    })
                    .content(
                        TextBlock::new()
                            .text(label)
                            .font_size(11.5)
                            .margin(Thickness::xy(4.0, 3.0)),
                    );
                elements.push(block.into());
                continue;
            }
            TreemapEntry::Free(free_bytes) => {
                let label = if node.rect.width > 60.0 && node.rect.height > 28.0 {
                    format!("Free\n{}", format_bytes(free_bytes))
                } else {
                    String::new()
                };
                let click_sender = sender.clone();
                let block = Border::new()
                    .width(node.rect.width.max(2.0))
                    .height(node.rect.height.max(2.0))
                    .background(Brush::Solid(Color::argb(255, 45, 45, 50)))
                    .border_brush(Brush::Solid(Color::argb(200, 30, 30, 34)))
                    .border_thickness(Thickness::uniform(1.0))
                    .corner_radius(CornerRadius::uniform(3.0))
                    .canvas_left(node.rect.x)
                    .canvas_top(node.rect.y)
                    .on_pointer_pressed(move |_| {
                        click_sender(RammapMessage::SelectProcess(None, None));
                    })
                    .content(
                        TextBlock::new()
                            .text(label)
                            .font_size(11.5)
                            .margin(Thickness::xy(4.0, 3.0)),
                    );
                elements.push(block.into());
                continue;
            }
        };
        let selected = app.selected_group_title.as_ref() == Some(&group.title);
        let background = if selected {
            Color::argb(255, 240, 160, 40)
        } else {
            color_for_category(group.category, index as u32)
        };
        let border = if selected {
            Color::argb(255, 255, 255, 255)
        } else {
            Color::argb(200, 20, 20, 20)
        };
        let value = format_bytes(group_metric_bytes(&group, metric));
        let label = if node.rect.width > 60.0 && node.rect.height > 28.0 {
            if node.rect.height > 44.0 && node.rect.width > 80.0 {
                format!("{}\n{}", group.title, value)
            } else {
                format!("{} ({})", group.title, value)
            }
        } else if node.rect.width > 35.0 && node.rect.height > 18.0 {
            group.title.clone()
        } else {
            String::new()
        };
        let first = group.items.first().cloned();
        let info = group_info(app.tab, &group);
        let click_sender = sender.clone();
        let block = Border::new()
            .width(node.rect.width.max(2.0))
            .height(node.rect.height.max(2.0))
            .background(Brush::Solid(background))
            .border_brush(Brush::Solid(border))
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(CornerRadius::uniform(3.0))
            .canvas_left(node.rect.x)
            .canvas_top(node.rect.y)
            .on_pointer_pressed(move |_| {
                click_sender(RammapMessage::SelectProcess(
                    first.clone(),
                    Some(info.clone()),
                ));
            })
            .content(
                TextBlock::new()
                    .text(label)
                    .font_size(if node.rect.height > 38.0 { 11.5 } else { 9.5 })
                    .margin(Thickness::xy(4.0, 3.0)),
            );
        elements.push(block.into());
    }
    wrap_canvas(
        Canvas::new()
            .width(canvas_width)
            .height(canvas_height)
            .children(elements)
            .into(),
    )
}
