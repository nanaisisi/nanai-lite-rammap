use super::content_core::{group_info, sort_groups, sort_processes};
use crate::category::{classify_process, group_by_category, group_by_name};
use crate::memory::{format_bytes, ProcessMemoryEntry};
use crate::ui::app_core::RammapApp;
use crate::ui::types::{GroupMode, RammapMessage};
use windows_reactor::*;

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

pub(super) fn individual_list<S: Fn(RammapMessage) + Clone + 'static>(
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

pub(super) fn grouped_list<S: Fn(RammapMessage) + Clone + 'static>(
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
