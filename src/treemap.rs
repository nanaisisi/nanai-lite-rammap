#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct TreemapItem<T> {
    pub value: f64,
    pub data: T,
}

#[derive(Debug, Clone)]
pub struct TreemapNode<T> {
    pub rect: Rect,
    pub data: T,
}

/// Squarified Treemap layout algorithm (similar to WizTree / SpaceMonger / WinDirStat)
pub fn layout_treemap<T: Clone>(items: &[TreemapItem<T>], bounds: Rect) -> Vec<TreemapNode<T>> {
    if items.is_empty() || bounds.width <= 0.0 || bounds.height <= 0.0 {
        return Vec::new();
    }

    let total_value: f64 = items.iter().map(|item| item.value.max(0.0)).sum();
    if total_value <= 0.0 {
        return Vec::new();
    }

    // Sort descending by value
    let mut sorted_items = items.to_vec();
    sorted_items.sort_by(|a, b| {
        b.value
            .partial_cmp(&a.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total_area = bounds.width * bounds.height;
    let mut normalized_items: Vec<(f64, T)> = sorted_items
        .into_iter()
        .map(|item| ((item.value.max(0.0) / total_value) * total_area, item.data))
        .collect();

    let mut result = Vec::new();
    squarify(&mut normalized_items, &mut Vec::new(), bounds, &mut result);
    result
}

fn squarify<T: Clone>(
    children: &mut Vec<(f64, T)>,
    row: &mut Vec<(f64, T)>,
    mut bounds: Rect,
    result: &mut Vec<TreemapNode<T>>,
) {
    if children.is_empty() {
        if !row.is_empty() {
            layout_row(row, bounds, result);
        }
        return;
    }

    if bounds.width <= 1.0 || bounds.height <= 1.0 {
        return;
    }

    let shortest_side = bounds.width.min(bounds.height);
    let next_item = &children[0];

    let mut candidate_row = row.clone();
    candidate_row.push(next_item.clone());

    if row.is_empty()
        || worst_aspect_ratio(row, shortest_side)
            >= worst_aspect_ratio(&candidate_row, shortest_side)
    {
        let item = children.remove(0);
        row.push(item);
        squarify(children, row, bounds, result);
    } else {
        let used_bounds = layout_row(row, bounds, result);
        row.clear();
        bounds = remaining_bounds(bounds, used_bounds);
        squarify(children, row, bounds, result);
    }
}

fn worst_aspect_ratio<T>(row: &[(f64, T)], side: f64) -> f64 {
    if row.is_empty() || side <= 0.0 {
        return f64::MAX;
    }
    let sum: f64 = row.iter().map(|(area, _)| *area).sum();
    if sum <= 0.0 {
        return f64::MAX;
    }
    let side_sq = side * side;
    let sum_sq = sum * sum;

    let mut max_ratio: f64 = 0.0;
    for &(area, _) in row {
        if area <= 0.0 {
            continue;
        }
        let r1 = (side_sq * area) / sum_sq;
        let r2 = sum_sq / (side_sq * area);
        let ratio = r1.max(r2);
        if ratio > max_ratio {
            max_ratio = ratio;
        }
    }
    max_ratio
}

fn layout_row<T: Clone>(row: &[(f64, T)], bounds: Rect, result: &mut Vec<TreemapNode<T>>) -> Rect {
    let row_area: f64 = row.iter().map(|(area, _)| *area).sum();
    if row_area <= 0.0 {
        return bounds;
    }

    let is_horizontal = bounds.width >= bounds.height;
    if is_horizontal {
        let row_width = (row_area / bounds.height).min(bounds.width);
        let mut current_y = bounds.y;

        for (area, data) in row {
            let item_height = if row_area > 0.0 {
                (area / row_width).min(bounds.y + bounds.height - current_y)
            } else {
                0.0
            };

            result.push(TreemapNode {
                rect: Rect {
                    x: bounds.x,
                    y: current_y,
                    width: row_width.max(1.0),
                    height: item_height.max(1.0),
                },
                data: data.clone(),
            });
            current_y += item_height;
        }

        Rect {
            x: bounds.x,
            y: bounds.y,
            width: row_width,
            height: bounds.height,
        }
    } else {
        let row_height = (row_area / bounds.width).min(bounds.height);
        let mut current_x = bounds.x;

        for (area, data) in row {
            let item_width = if row_area > 0.0 {
                (area / row_height).min(bounds.x + bounds.width - current_x)
            } else {
                0.0
            };

            result.push(TreemapNode {
                rect: Rect {
                    x: current_x,
                    y: bounds.y,
                    width: item_width.max(1.0),
                    height: row_height.max(1.0),
                },
                data: data.clone(),
            });
            current_x += item_width;
        }

        Rect {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: row_height,
        }
    }
}

fn remaining_bounds(total: Rect, used: Rect) -> Rect {
    if total.width >= total.height {
        Rect {
            x: total.x + used.width,
            y: total.y,
            width: (total.width - used.width).max(0.0),
            height: total.height,
        }
    } else {
        Rect {
            x: total.x,
            y: total.y + used.height,
            width: total.width,
            height: (total.height - used.height).max(0.0),
        }
    }
}
