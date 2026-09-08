use crate::category::{ProcessGroup, classify_process, group_by_category, group_by_name};
use crate::memory::{ProcessMemoryEntry, format_bytes};
use crate::treemap::{Rect, TreemapItem, layout_treemap};
use crate::ui::app_core::RammapApp;
use crate::ui::theme::{color_for_category, wrap_canvas};
use crate::ui::types::{GroupMode, MemoryMetric, MemoryTab, RammapMessage, ViewMode};
use windows_reactor::*;

const CANVAS_WIDTH: f64 = 1000.0;
const CANVAS_HEIGHT: f64 = 540.0;

pub fn build<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
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

fn metric_value(p: &ProcessMemoryEntry, metric: MemoryMetric) -> f64 {
    match metric {
        MemoryMetric::WorkingSet => p.working_set_bytes as f64,
        MemoryMetric::PrivateWs => p.private_bytes as f64,
        MemoryMetric::GpuDedicated => p.gpu_dedicated_bytes as f64,
        MemoryMetric::GpuShared => p.gpu_shared_bytes as f64,
    }
}

fn group_metric_value(g: &ProcessGroup<ProcessMemoryEntry>, metric: MemoryMetric) -> f64 {
    match metric {
        MemoryMetric::WorkingSet => g.total_working_set as f64,
        MemoryMetric::PrivateWs => g.total_private as f64,
        MemoryMetric::GpuDedicated => g.total_gpu_dedicated as f64,
        MemoryMetric::GpuShared => g.total_gpu_shared as f64,
    }
}

fn metric_bytes(p: &ProcessMemoryEntry, metric: MemoryMetric) -> u64 {
    metric_value(p, metric) as u64
}

fn group_metric_bytes(g: &ProcessGroup<ProcessMemoryEntry>, metric: MemoryMetric) -> u64 {
    group_metric_value(g, metric) as u64
}

fn bounds() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width: CANVAS_WIDTH,
        height: CANVAS_HEIGHT,
    }
}

fn individual_treemap<S: Fn(RammapMessage) + Clone + 'static>(
    app: &RammapApp,
    sender: S,
    processes: Vec<ProcessMemoryEntry>,
) -> View {
    let items = processes
        .into_iter()
        .map(|p| TreemapItem {
            value: metric_value(&p, app.metric),
            data: p,
        })
        .collect::<Vec<_>>();
    let mut elements = Vec::new();

    for node in layout_treemap(&items, bounds()) {
        let p = node.data;
        let pid = p.pid;
        let label_value = format_bytes(metric_bytes(&p, app.metric));
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
        elements.push(KeyedView::new(pid, block));
    }
    wrap_canvas(
        Canvas::new()
            .width(CANVAS_WIDTH)
            .height(CANVAS_HEIGHT)
            .keyed_children(elements),
    )
}

fn grouped_treemap<S: Fn(RammapMessage) + Clone + 'static>(
    app: &RammapApp,
    sender: S,
    processes: Vec<ProcessMemoryEntry>,
) -> View {
    let groups = if app.group_mode == GroupMode::ByName {
        group_by_name(&processes)
    } else {
        group_by_category(&processes)
    };
    let items = groups
        .into_iter()
        .map(|g| TreemapItem {
            value: group_metric_value(&g, app.metric),
            data: g,
        })
        .collect::<Vec<_>>();
    let mut elements = Vec::new();

    for (index, node) in layout_treemap(&items, bounds()).into_iter().enumerate() {
        let group = node.data;
        let key = group.key.clone();
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
        let value = format_bytes(group_metric_bytes(&group, app.metric));
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
        elements.push(KeyedView::new(key, block));
    }
    wrap_canvas(
        Canvas::new()
            .width(CANVAS_WIDTH)
            .height(CANVAS_HEIGHT)
            .keyed_children(elements),
    )
}

fn group_info(tab: MemoryTab, group: &ProcessGroup<ProcessMemoryEntry>) -> String {
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

fn sort_processes(processes: &mut [ProcessMemoryEntry], metric: MemoryMetric) {
    processes.sort_by_key(|p| std::cmp::Reverse(metric_bytes(p, metric)));
}

fn sort_groups(groups: &mut [ProcessGroup<ProcessMemoryEntry>], metric: MemoryMetric) {
    groups.sort_by_key(|g| std::cmp::Reverse(group_metric_bytes(g, metric)));
}

fn process_header() -> View {
    Border::new().padding(Thickness::xy(8.0, 6.0)).content(
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(16.0)
            .children((
                TextBlock::new()
                    .text("PID")
                    .width(60.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("Process Name")
                    .width(190.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("Category")
                    .width(130.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("Working Set")
                    .width(100.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("Private WS")
                    .width(100.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("GPU VRAM")
                    .width(100.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("GPU Shared")
                    .width(100.0)
                    .font_weight(FontWeight::BOLD),
            )),
    )
}

fn group_header() -> View {
    Border::new().padding(Thickness::xy(8.0, 6.0)).content(
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(16.0)
            .children((
                TextBlock::new()
                    .text("Group / Category")
                    .width(260.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("Count")
                    .width(60.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("Total WS")
                    .width(100.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("Total Private")
                    .width(100.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("Total VRAM")
                    .width(100.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("Total Shared")
                    .width(100.0)
                    .font_weight(FontWeight::BOLD),
                TextBlock::new()
                    .text("Category")
                    .width(130.0)
                    .font_weight(FontWeight::BOLD),
            )),
    )
}

fn individual_list<S: Fn(RammapMessage) + Clone + 'static>(
    app: &RammapApp,
    sender: S,
    mut processes: Vec<ProcessMemoryEntry>,
) -> View {
    sort_processes(&mut processes, app.metric);
    let mut rows = Vec::new();
    for p in processes.into_iter().take(150) {
        let pid = p.pid;
        let category = classify_process(&p.name);
        let selected = app.selected_process.as_ref().is_some_and(|s| s.pid == pid);
        let click_sender = sender.clone();
        let selected_process = p.clone();
        let row = Border::new()
            .padding(Thickness::xy(8.0, 3.0))
            .background(if selected {
                Brush::Solid(Color::argb(80, 0, 120, 215))
            } else {
                Brush::Solid(Color::transparent())
            })
            .on_pointer_pressed(move |_| {
                click_sender(RammapMessage::SelectProcess(
                    Some(selected_process.clone()),
                    None,
                ));
            })
            .content(
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(16.0)
                    .children((
                        TextBlock::new().text(pid.to_string()).width(60.0),
                        TextBlock::new().text(p.name).width(190.0),
                        TextBlock::new().text(category.label()).width(130.0),
                        TextBlock::new()
                            .text(format_bytes(p.working_set_bytes))
                            .width(100.0),
                        TextBlock::new()
                            .text(format_bytes(p.private_bytes))
                            .width(100.0),
                        TextBlock::new()
                            .text(format_bytes(p.gpu_dedicated_bytes))
                            .width(100.0),
                        TextBlock::new()
                            .text(format_bytes(p.gpu_shared_bytes))
                            .width(100.0),
                    )),
            );
        rows.push(KeyedView::new(pid, row));
    }
    let content = ScrollViewer::new()
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .height(500.0)
        .content(StackPanel::new().spacing(2.0).keyed_children(rows));
    StackPanel::new()
        .spacing(4.0)
        .children((process_header(), content))
}

fn grouped_list<S: Fn(RammapMessage) + Clone + 'static>(
    app: &RammapApp,
    sender: S,
    processes: Vec<ProcessMemoryEntry>,
) -> View {
    let mut groups = if app.group_mode == GroupMode::ByName {
        group_by_name(&processes)
    } else {
        group_by_category(&processes)
    };
    sort_groups(&mut groups, app.metric);
    let mut rows = Vec::new();
    for group in groups {
        let key = group.key.clone();
        let selected = app.selected_group_title.as_ref() == Some(&group.title);
        let click_sender = sender.clone();
        let first = group.items.first().cloned();
        let info = group_info(app.tab, &group);
        let row = Border::new()
            .padding(Thickness::xy(8.0, 4.0))
            .background(if selected {
                Brush::Solid(Color::argb(80, 0, 120, 215))
            } else {
                Brush::Solid(Color::transparent())
            })
            .on_pointer_pressed(move |_| {
                click_sender(RammapMessage::SelectProcess(
                    first.clone(),
                    Some(info.clone()),
                ));
            })
            .content(
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(16.0)
                    .children((
                        TextBlock::new().text(group.title).width(260.0),
                        TextBlock::new()
                            .text(format!("{} inst", group.items.len()))
                            .width(60.0),
                        TextBlock::new()
                            .text(format_bytes(group.total_working_set))
                            .width(100.0),
                        TextBlock::new()
                            .text(format_bytes(group.total_private))
                            .width(100.0),
                        TextBlock::new()
                            .text(format_bytes(group.total_gpu_dedicated))
                            .width(100.0),
                        TextBlock::new()
                            .text(format_bytes(group.total_gpu_shared))
                            .width(100.0),
                        TextBlock::new().text(group.category.label()).width(130.0),
                    )),
            );
        rows.push(KeyedView::new(key, row));
    }
    let content = ScrollViewer::new()
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .height(500.0)
        .content(StackPanel::new().spacing(2.0).keyed_children(rows));
    StackPanel::new()
        .spacing(4.0)
        .children((group_header(), content))
}
