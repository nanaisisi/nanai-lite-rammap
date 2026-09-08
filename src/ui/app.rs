use crate::category::{ProcessGroup, classify_process, group_by_category, group_by_name};
use crate::gpu::{GpuAdapterInfo, collect_all_process_gpu_memory, collect_gpu_adapters};
use crate::memory::{
    ProcessMemoryEntry, SystemMemorySummary, collect_process_memory, collect_system_summary,
    format_bytes,
};
use crate::treemap::{Rect, TreemapItem, layout_treemap};
use crate::ui::theme::{color_for_category, wrap_canvas};
use crate::ui::types::{AppInput, GroupMode, MemoryMetric, MemoryTab, RammapMessage, ViewMode};
use windows_reactor::*;

pub struct RammapApp {
    tab: MemoryTab,
    summary: SystemMemorySummary,
    gpu_adapters: Vec<GpuAdapterInfo>,
    processes: Vec<ProcessMemoryEntry>,
    search_query: String,
    view_mode: ViewMode,
    group_mode: GroupMode,
    metric: MemoryMetric,
    selected_process: Option<ProcessMemoryEntry>,
    selected_group_title: Option<String>,
}

fn fetch_all_processes(gpu_adapters: &[GpuAdapterInfo]) -> Vec<ProcessMemoryEntry> {
    let mut processes = collect_process_memory();
    let pids: Vec<u32> = processes.iter().map(|p| p.pid).collect();
    let gpu_map = collect_all_process_gpu_memory(&pids, gpu_adapters);

    for p in &mut processes {
        if let Some(gpu_info) = gpu_map.get(&p.pid) {
            p.gpu_dedicated_bytes = gpu_info.dedicated_bytes;
            p.gpu_shared_bytes = gpu_info.shared_bytes;
        }
    }
    processes
}

impl Component for RammapApp {
    type Input = AppInput;
    type Message = RammapMessage;

    fn create(_input: &Self::Input, _context: &ComponentContext<Self>) -> Self {
        let summary = collect_system_summary();
        let gpu_adapters = collect_gpu_adapters();
        let processes = fetch_all_processes(&gpu_adapters);
        Self {
            tab: MemoryTab::SystemRam,
            summary,
            gpu_adapters,
            processes,
            search_query: String::new(),
            view_mode: ViewMode::Treemap,
            group_mode: GroupMode::ByName,
            metric: MemoryMetric::WorkingSet,
            selected_process: None,
            selected_group_title: None,
        }
    }

    fn update(&mut self, message: Self::Message, _context: &ComponentContext<Self>) {
        match message {
            RammapMessage::Refresh => {
                self.summary = collect_system_summary();
                self.gpu_adapters = collect_gpu_adapters();
                self.processes = fetch_all_processes(&self.gpu_adapters);
                if let Some(ref sel) = self.selected_process {
                    let pid = sel.pid;
                    self.selected_process = self.processes.iter().find(|p| p.pid == pid).cloned();
                }
            }
            RammapMessage::SetTab(tab) => {
                self.tab = tab;
                if tab == MemoryTab::GpuVram
                    && self.metric != MemoryMetric::GpuDedicated
                    && self.metric != MemoryMetric::GpuShared
                {
                    self.metric = MemoryMetric::GpuDedicated;
                } else if tab == MemoryTab::SystemRam
                    && (self.metric == MemoryMetric::GpuDedicated
                        || self.metric == MemoryMetric::GpuShared)
                {
                    self.metric = MemoryMetric::WorkingSet;
                }
            }
            RammapMessage::SearchChanged(query) => {
                self.search_query = query;
            }
            RammapMessage::SetViewMode(mode) => {
                self.view_mode = mode;
            }
            RammapMessage::SetGroupMode(mode) => {
                self.group_mode = mode;
            }
            RammapMessage::ToggleMetric => {
                self.metric = match self.tab {
                    MemoryTab::SystemRam => match self.metric {
                        MemoryMetric::WorkingSet => MemoryMetric::PrivateWs,
                        _ => MemoryMetric::WorkingSet,
                    },
                    MemoryTab::GpuVram => match self.metric {
                        MemoryMetric::GpuDedicated => MemoryMetric::GpuShared,
                        _ => MemoryMetric::GpuDedicated,
                    },
                };
            }
            RammapMessage::SelectProcess(p, grp_title) => {
                self.selected_process = p;
                self.selected_group_title = grp_title;
            }
        }
    }

    fn view(&self, _input: &Self::Input, context: &mut ViewContext<Self>) -> View {
        let sender = context.sender();

        let mut filtered_processes = self.processes.clone();
        let query = self.search_query.to_lowercase();
        if !query.is_empty() {
            filtered_processes.retain(|p| {
                let cat = classify_process(&p.name);
                p.name.to_lowercase().contains(&query)
                    || p.pid.to_string().contains(&query)
                    || cat.label().to_lowercase().contains(&query)
            });
        }

        // Header
        let s_refresh = sender.clone();
        let s_tab_ram = sender.clone();
        let s_tab_gpu = sender.clone();
        let s_mode_tree = sender.clone();
        let s_mode_list = sender.clone();
        let s_grp_indiv = sender.clone();
        let s_grp_name = sender.clone();
        let s_grp_cat = sender.clone();
        let s_metric = sender.clone();
        let s_search = sender.clone();

        let title = TextBlock::new()
            .text("Nanai Lite Rammap (WizTree + GPU Edition)")
            .font_size(22.0)
            .font_weight(FontWeight::BOLD);

        let gpu_summary_str = if self.gpu_adapters.is_empty() {
            "GPU: No DirectX 11/12 GPU detected".to_string()
        } else {
            self.gpu_adapters
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    let vram_used = if g.dedicated_usage_bytes > 0 {
                        format_bytes(g.dedicated_usage_bytes)
                    } else {
                        "0 B".to_string()
                    };
                    let vram_total = format_bytes(g.dedicated_vram_bytes);
                    let shared_used = if g.shared_usage_bytes > 0 {
                        format_bytes(g.shared_usage_bytes)
                    } else {
                        "0 B".to_string()
                    };
                    let shared_total = format_bytes(g.shared_ram_bytes);
                    format!(
                        "GPU {}: {} | Dedicated VRAM: {} / {} | Shared: {} / {}",
                        i, g.name, vram_used, vram_total, shared_used, shared_total
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ")
        };

        let subtitle_text = match self.tab {
            MemoryTab::SystemRam => format!(
                "Processes: {} | Physical RAM: {} (Used: {} / {}%) | Commit: {} / {}",
                self.summary.process_count,
                format_bytes(self.summary.total_phys_bytes),
                format_bytes(self.summary.used_phys_bytes),
                self.summary.memory_load_pct,
                format_bytes(self.summary.commit_total_bytes),
                format_bytes(self.summary.commit_limit_bytes),
            ),
            MemoryTab::GpuVram => format!(
                "Processes: {} | {}",
                self.summary.process_count, gpu_summary_str
            ),
        };

        let subtitle = TextBlock::new().text(subtitle_text).font_size(12.0);

        let tab_ram_btn = Button::new()
            .on_click(move || {
                s_tab_ram.send(RammapMessage::SetTab(MemoryTab::SystemRam));
            })
            .content(TextBlock::new().text(if self.tab == MemoryTab::SystemRam {
                "💻 System RAM (Active)"
            } else {
                "💻 System RAM"
            }));

        let tab_gpu_btn = Button::new()
            .on_click(move || {
                s_tab_gpu.send(RammapMessage::SetTab(MemoryTab::GpuVram));
            })
            .content(TextBlock::new().text(if self.tab == MemoryTab::GpuVram {
                "🎮 GPU VRAM (Active)"
            } else {
                "🎮 GPU VRAM"
            }));

        let refresh_btn = Button::new()
            .on_click(move || {
                s_refresh.send(RammapMessage::Refresh);
            })
            .content(TextBlock::new().text("🔄 Refresh"));

        let mode_tree_btn = Button::new()
            .on_click(move || {
                s_mode_tree.send(RammapMessage::SetViewMode(ViewMode::Treemap));
            })
            .content(
                TextBlock::new().text(if self.view_mode == ViewMode::Treemap {
                    "📊 2D Tree (Active)"
                } else {
                    "📊 2D Tree"
                }),
            );

        let mode_list_btn = Button::new()
            .on_click(move || {
                s_mode_list.send(RammapMessage::SetViewMode(ViewMode::List));
            })
            .content(TextBlock::new().text(if self.view_mode == ViewMode::List {
                "📋 List (Active)"
            } else {
                "📋 List"
            }));

        let grp_indiv_btn = Button::new()
            .on_click(move || {
                s_grp_indiv.send(RammapMessage::SetGroupMode(GroupMode::Individual));
            })
            .content(
                TextBlock::new().text(if self.group_mode == GroupMode::Individual {
                    "🔹 Individual (Active)"
                } else {
                    "🔹 Individual"
                }),
            );

        let grp_name_btn = Button::new()
            .on_click(move || {
                s_grp_name.send(RammapMessage::SetGroupMode(GroupMode::ByName));
            })
            .content(
                TextBlock::new().text(if self.group_mode == GroupMode::ByName {
                    "🗂️ By Name (Active)"
                } else {
                    "🗂️ By Name"
                }),
            );

        let grp_cat_btn = Button::new()
            .on_click(move || {
                s_grp_cat.send(RammapMessage::SetGroupMode(GroupMode::ByCategory));
            })
            .content(
                TextBlock::new().text(if self.group_mode == GroupMode::ByCategory {
                    "🏷️ By Category (Active)"
                } else {
                    "🏷️ By Category"
                }),
            );

        let metric_label = match self.metric {
            MemoryMetric::WorkingSet => "Metric: Working Set",
            MemoryMetric::PrivateWs => "Metric: Private (Commit)",
            MemoryMetric::GpuDedicated => "Metric: Dedicated VRAM",
            MemoryMetric::GpuShared => "Metric: Shared GPU Memory",
        };

        let metric_btn = Button::new()
            .on_click(move || {
                s_metric.send(RammapMessage::ToggleMetric);
            })
            .content(TextBlock::new().text(metric_label));

        let header_actions = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                tab_ram_btn,
                tab_gpu_btn,
                refresh_btn,
                mode_tree_btn,
                mode_list_btn,
                grp_name_btn,
                grp_cat_btn,
                grp_indiv_btn,
                metric_btn,
            ));

        let header = StackPanel::new()
            .spacing(4.0)
            .children((title, subtitle, header_actions));

        // Search & Status bar
        let search_box = TextBox::new()
            .placeholder_text("Filter PID, name or category...")
            .text(self.search_query.clone())
            .width(280.0)
            .on_text_changed(move |text: String| {
                s_search.send(RammapMessage::SearchChanged(text));
            });

        let selected_info_text = if let Some(ref title) = self.selected_group_title {
            format!("🎯 Selected Group: {}", title)
        } else if let Some(ref p) = self.selected_process {
            let cat = classify_process(&p.name);
            format!(
                "🎯 [{}] {} ({}) | WS: {} | Private: {} | GPU VRAM: {} | GPU Shared: {}",
                p.pid,
                p.name,
                cat.label(),
                format_bytes(p.working_set_bytes),
                format_bytes(p.private_bytes),
                format_bytes(p.gpu_dedicated_bytes),
                format_bytes(p.gpu_shared_bytes),
            )
        } else {
            "💡 Click any 2D tile or table row to inspect details. Switch between System RAM and GPU VRAM tabs above.".to_string()
        };

        let selected_info = TextBlock::new()
            .text(selected_info_text)
            .font_size(12.0)
            .vertical_alignment(VerticalAlignment::Center);

        let search_panel = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((search_box, selected_info));

        let canvas_width = 1000.0;
        let canvas_height = 540.0;

        // Content Rendering
        let content_view: View = match self.view_mode {
            ViewMode::Treemap => match self.group_mode {
                GroupMode::Individual => {
                    let treemap_items: Vec<TreemapItem<ProcessMemoryEntry>> = filtered_processes
                        .iter()
                        .map(|p| {
                            let val = match self.metric {
                                MemoryMetric::WorkingSet => p.working_set_bytes as f64,
                                MemoryMetric::PrivateWs => p.private_bytes as f64,
                                MemoryMetric::GpuDedicated => p.gpu_dedicated_bytes as f64,
                                MemoryMetric::GpuShared => p.gpu_shared_bytes as f64,
                            };
                            TreemapItem {
                                value: val,
                                data: p.clone(),
                            }
                        })
                        .collect();

                    let bounds = Rect {
                        x: 0.0,
                        y: 0.0,
                        width: canvas_width,
                        height: canvas_height,
                    };
                    let nodes = layout_treemap(&treemap_items, bounds);

                    let mut keyed_elements = Vec::new();
                    for node in nodes {
                        let p = node.data;
                        let pid = p.pid;
                        let p_clone = p.clone();
                        let s_sel = sender.clone();

                        let val_str = match self.metric {
                            MemoryMetric::WorkingSet => format_bytes(p.working_set_bytes),
                            MemoryMetric::PrivateWs => format_bytes(p.private_bytes),
                            MemoryMetric::GpuDedicated => format_bytes(p.gpu_dedicated_bytes),
                            MemoryMetric::GpuShared => format_bytes(p.gpu_shared_bytes),
                        };

                        let is_selected = self
                            .selected_process
                            .as_ref()
                            .is_some_and(|sel| sel.pid == pid);
                        let cat = classify_process(&p.name);
                        let bg_color = if is_selected {
                            Color::argb(255, 240, 160, 40)
                        } else {
                            color_for_category(cat, pid)
                        };

                        let border_color = if is_selected {
                            Color::argb(255, 255, 255, 255)
                        } else {
                            Color::argb(180, 20, 20, 20)
                        };

                        let label_text = if node.rect.width > 55.0 && node.rect.height > 26.0 {
                            if node.rect.height > 42.0 && node.rect.width > 70.0 {
                                format!("{}\n{}", p.name, val_str)
                            } else {
                                format!("{} ({})", p.name, val_str)
                            }
                        } else if node.rect.width > 30.0 && node.rect.height > 18.0 {
                            p.name.clone()
                        } else {
                            String::new()
                        };

                        let block = Border::new()
                            .width(node.rect.width.max(2.0))
                            .height(node.rect.height.max(2.0))
                            .background(Brush::Solid(bg_color))
                            .border_brush(Brush::Solid(border_color))
                            .border_thickness(Thickness::uniform(1.0))
                            .corner_radius(CornerRadius::uniform(2.0))
                            .canvas_left(node.rect.x)
                            .canvas_top(node.rect.y)
                            .on_pointer_pressed(move |_| {
                                s_sel.send(RammapMessage::SelectProcess(
                                    Some(p_clone.clone()),
                                    None,
                                ));
                            })
                            .content(
                                TextBlock::new()
                                    .text(label_text)
                                    .font_size(if node.rect.height > 35.0 { 11.0 } else { 9.5 })
                                    .margin(Thickness::xy(3.0, 2.0)),
                            );

                        keyed_elements.push(KeyedView::new(pid, block));
                    }

                    let canvas = Canvas::new()
                        .width(canvas_width)
                        .height(canvas_height)
                        .keyed_children(keyed_elements);

                    wrap_canvas(canvas)
                }
                GroupMode::ByName | GroupMode::ByCategory => {
                    let groups = if self.group_mode == GroupMode::ByName {
                        group_by_name(&filtered_processes)
                    } else {
                        group_by_category(&filtered_processes)
                    };

                    let treemap_items: Vec<TreemapItem<ProcessGroup<ProcessMemoryEntry>>> = groups
                        .into_iter()
                        .map(|g| {
                            let val = match self.metric {
                                MemoryMetric::WorkingSet => g.total_working_set as f64,
                                MemoryMetric::PrivateWs => g.total_private as f64,
                                MemoryMetric::GpuDedicated => g.total_gpu_dedicated as f64,
                                MemoryMetric::GpuShared => g.total_gpu_shared as f64,
                            };
                            TreemapItem {
                                value: val,
                                data: g,
                            }
                        })
                        .collect();

                    let bounds = Rect {
                        x: 0.0,
                        y: 0.0,
                        width: canvas_width,
                        height: canvas_height,
                    };
                    let nodes = layout_treemap(&treemap_items, bounds);

                    let mut keyed_elements = Vec::new();
                    for (idx, node) in nodes.into_iter().enumerate() {
                        let g = node.data;
                        let key = g.key.clone();
                        let s_sel = sender.clone();

                        let total_val = match self.metric {
                            MemoryMetric::WorkingSet => g.total_working_set,
                            MemoryMetric::PrivateWs => g.total_private,
                            MemoryMetric::GpuDedicated => g.total_gpu_dedicated,
                            MemoryMetric::GpuShared => g.total_gpu_shared,
                        };
                        let val_str = format_bytes(total_val);

                        let is_selected = self.selected_group_title.as_ref() == Some(&g.title);
                        let bg_color = if is_selected {
                            Color::argb(255, 240, 160, 40)
                        } else {
                            color_for_category(g.category, idx as u32)
                        };

                        let border_color = if is_selected {
                            Color::argb(255, 255, 255, 255)
                        } else {
                            Color::argb(200, 20, 20, 20)
                        };

                        let label_text = if node.rect.width > 60.0 && node.rect.height > 28.0 {
                            if node.rect.height > 44.0 && node.rect.width > 80.0 {
                                format!("{}\n{}", g.title, val_str)
                            } else {
                                format!("{} ({})", g.title, val_str)
                            }
                        } else if node.rect.width > 35.0 && node.rect.height > 18.0 {
                            g.title.clone()
                        } else {
                            String::new()
                        };

                        let first_item = g.items.first().cloned();
                        let full_info = match self.tab {
                            MemoryTab::SystemRam => format!(
                                "{} | Total WS: {} | Total Private: {} | Count: {}",
                                g.title,
                                format_bytes(g.total_working_set),
                                format_bytes(g.total_private),
                                g.items.len()
                            ),
                            MemoryTab::GpuVram => format!(
                                "{} | Total Dedicated VRAM: {} | Total Shared: {} | Count: {}",
                                g.title,
                                format_bytes(g.total_gpu_dedicated),
                                format_bytes(g.total_gpu_shared),
                                g.items.len()
                            ),
                        };

                        let block = Border::new()
                            .width(node.rect.width.max(2.0))
                            .height(node.rect.height.max(2.0))
                            .background(Brush::Solid(bg_color))
                            .border_brush(Brush::Solid(border_color))
                            .border_thickness(Thickness::uniform(1.0))
                            .corner_radius(CornerRadius::uniform(3.0))
                            .canvas_left(node.rect.x)
                            .canvas_top(node.rect.y)
                            .on_pointer_pressed(move |_| {
                                s_sel.send(RammapMessage::SelectProcess(
                                    first_item.clone(),
                                    Some(full_info.clone()),
                                ));
                            })
                            .content(
                                TextBlock::new()
                                    .text(label_text)
                                    .font_size(if node.rect.height > 38.0 { 11.5 } else { 9.5 })
                                    .margin(Thickness::xy(4.0, 3.0)),
                            );

                        keyed_elements.push(KeyedView::new(key, block));
                    }

                    let canvas = Canvas::new()
                        .width(canvas_width)
                        .height(canvas_height)
                        .keyed_children(keyed_elements);

                    wrap_canvas(canvas)
                }
            },
            ViewMode::List => match self.group_mode {
                GroupMode::Individual => {
                    let mut sorted = filtered_processes;
                    match self.metric {
                        MemoryMetric::WorkingSet => {
                            sorted.sort_by_key(|a| std::cmp::Reverse(a.working_set_bytes))
                        }
                        MemoryMetric::PrivateWs => {
                            sorted.sort_by_key(|a| std::cmp::Reverse(a.private_bytes))
                        }
                        MemoryMetric::GpuDedicated => {
                            sorted.sort_by_key(|a| std::cmp::Reverse(a.gpu_dedicated_bytes))
                        }
                        MemoryMetric::GpuShared => {
                            sorted.sort_by_key(|a| std::cmp::Reverse(a.gpu_shared_bytes))
                        }
                    }

                    let table_header = Border::new().padding(Thickness::xy(8.0, 6.0)).content(
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
                    );

                    let mut keyed_rows = Vec::new();
                    for p in sorted.into_iter().take(150) {
                        let pid = p.pid;
                        let p_clone = p.clone();
                        let s_sel = sender.clone();
                        let cat = classify_process(&p.name);
                        let is_selected = self
                            .selected_process
                            .as_ref()
                            .is_some_and(|sel| sel.pid == pid);

                        let row_bg = if is_selected {
                            Brush::Solid(Color::argb(80, 0, 120, 215))
                        } else {
                            Brush::Solid(Color::transparent())
                        };

                        let row = Border::new()
                            .padding(Thickness::xy(8.0, 3.0))
                            .background(row_bg)
                            .on_pointer_pressed(move |_| {
                                s_sel.send(RammapMessage::SelectProcess(
                                    Some(p_clone.clone()),
                                    None,
                                ));
                            })
                            .content(
                                StackPanel::new()
                                    .orientation(Orientation::Horizontal)
                                    .spacing(16.0)
                                    .children((
                                        TextBlock::new().text(p.pid.to_string()).width(60.0),
                                        TextBlock::new().text(p.name).width(190.0),
                                        TextBlock::new().text(cat.label()).width(130.0),
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
                        keyed_rows.push(KeyedView::new(pid, row));
                    }

                    let process_rows_panel =
                        StackPanel::new().spacing(2.0).keyed_children(keyed_rows);

                    let scroll_content = ScrollViewer::new()
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .height(500.0)
                        .content(process_rows_panel);

                    StackPanel::new()
                        .spacing(4.0)
                        .children((table_header, scroll_content))
                }
                GroupMode::ByName | GroupMode::ByCategory => {
                    let mut groups = if self.group_mode == GroupMode::ByName {
                        group_by_name(&filtered_processes)
                    } else {
                        group_by_category(&filtered_processes)
                    };

                    match self.metric {
                        MemoryMetric::WorkingSet => {
                            groups.sort_by_key(|a| std::cmp::Reverse(a.total_working_set))
                        }
                        MemoryMetric::PrivateWs => {
                            groups.sort_by_key(|a| std::cmp::Reverse(a.total_private))
                        }
                        MemoryMetric::GpuDedicated => {
                            groups.sort_by_key(|a| std::cmp::Reverse(a.total_gpu_dedicated))
                        }
                        MemoryMetric::GpuShared => {
                            groups.sort_by_key(|a| std::cmp::Reverse(a.total_gpu_shared))
                        }
                    }

                    let table_header = Border::new().padding(Thickness::xy(8.0, 6.0)).content(
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
                    );

                    let mut keyed_rows = Vec::new();
                    for g in groups {
                        let key = g.key.clone();
                        let s_sel = sender.clone();
                        let is_selected = self.selected_group_title.as_ref() == Some(&g.title);
                        let first_item = g.items.first().cloned();
                        let full_info = match self.tab {
                            MemoryTab::SystemRam => format!(
                                "{} | Total WS: {} | Total Private: {} | Count: {}",
                                g.title,
                                format_bytes(g.total_working_set),
                                format_bytes(g.total_private),
                                g.items.len()
                            ),
                            MemoryTab::GpuVram => format!(
                                "{} | Total Dedicated VRAM: {} | Total Shared: {} | Count: {}",
                                g.title,
                                format_bytes(g.total_gpu_dedicated),
                                format_bytes(g.total_gpu_shared),
                                g.items.len()
                            ),
                        };

                        let row_bg = if is_selected {
                            Brush::Solid(Color::argb(80, 0, 120, 215))
                        } else {
                            Brush::Solid(Color::transparent())
                        };

                        let row = Border::new()
                            .padding(Thickness::xy(8.0, 4.0))
                            .background(row_bg)
                            .on_pointer_pressed(move |_| {
                                s_sel.send(RammapMessage::SelectProcess(
                                    first_item.clone(),
                                    Some(full_info.clone()),
                                ));
                            })
                            .content(
                                StackPanel::new()
                                    .orientation(Orientation::Horizontal)
                                    .spacing(16.0)
                                    .children((
                                        TextBlock::new().text(g.title).width(260.0),
                                        TextBlock::new()
                                            .text(format!("{} inst", g.items.len()))
                                            .width(60.0),
                                        TextBlock::new()
                                            .text(format_bytes(g.total_working_set))
                                            .width(100.0),
                                        TextBlock::new()
                                            .text(format_bytes(g.total_private))
                                            .width(100.0),
                                        TextBlock::new()
                                            .text(format_bytes(g.total_gpu_dedicated))
                                            .width(100.0),
                                        TextBlock::new()
                                            .text(format_bytes(g.total_gpu_shared))
                                            .width(100.0),
                                        TextBlock::new().text(g.category.label()).width(130.0),
                                    )),
                            );
                        keyed_rows.push(KeyedView::new(key, row));
                    }

                    let process_rows_panel =
                        StackPanel::new().spacing(2.0).keyed_children(keyed_rows);

                    let scroll_content = ScrollViewer::new()
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .height(500.0)
                        .content(process_rows_panel);

                    StackPanel::new()
                        .spacing(4.0)
                        .children((table_header, scroll_content))
                }
            },
        };

        StackPanel::new()
            .spacing(10.0)
            .margin(Thickness::uniform(14.0))
            .children((header, search_panel, content_view))
    }
}
