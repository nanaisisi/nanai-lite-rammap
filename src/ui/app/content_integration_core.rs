use super::content_core::{
    group_info, group_metric_bytes, group_metric_value, metric_bytes, metric_value,
};
use crate::category::{classify_process, group_by_category, group_by_name};
use crate::memory::{format_bytes, ProcessMemoryEntry};
use crate::treemap::{layout_treemap, Rect, TreemapItem};
use crate::ui::app_core::RammapApp;
use crate::ui::theme::{color_for_category, wrap_canvas};
use crate::ui::types::{GroupMode, RammapMessage};
use windows_reactor::*;

const CANVAS_WIDTH: f64 = 1000.0;
const CANVAS_HEIGHT: f64 = 540.0;

fn bounds() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width: CANVAS_WIDTH,
        height: CANVAS_HEIGHT,
    }
}

pub(super) fn individual_treemap<S: Fn(RammapMessage) + Clone + 'static>(
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

pub(super) fn grouped_treemap<S: Fn(RammapMessage) + Clone + 'static>(
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
