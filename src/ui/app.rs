#[path = "app/content_core.rs"]
mod content_core;
#[path = "app/content_integration.rs"]
mod content_integration;
#[path = "app/content_integration_core.rs"]
mod content_integration_core;
#[path = "app/content_integration_integration.rs"]
mod content_integration_integration;

use crate::category::classify_process;
use crate::memory::{ProcessMemoryEntry, format_bytes};
use crate::ui::types::{
    AppInput, GroupMode, MemoryTab, RammapMessage, SplitLayoutMode, ViewMode,
};
use windows_reactor::*;

pub use crate::ui::app_core::RammapApp;

impl Component for RammapApp {
    type Input = AppInput;
    type Message = RammapMessage;

    fn create(input: &Self::Input, _context: &ComponentContext<Self>) -> Self {
        Self::new(input)
    }
    fn update(&mut self, message: Self::Message, _context: &ComponentContext<Self>) {
        self.update_message(message);
    }

    fn view(&self, _input: &Self::Input, context: &mut ViewContext<Self>) -> View {
        let sender = context.sender();
        let send = {
            let sender = sender.clone();
            move |message| {
                let _ = sender.send(message);
            }
        };
        let content =
            content_integration_integration::build(self, send.clone(), self.filtered_processes());
        Border::new()
            .background(Brush::Solid(Color::argb(255, 24, 24, 26)))
            .content(
                ScrollViewer::new()
                    .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                    .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
                    .content(
                        StackPanel::new()
                            .spacing(10.0)
                            .margin(Thickness::uniform(14.0))
                            .children((self.header(send.clone()), self.search_panel(send), content)),
                    ),
            )
            .into()
    }
}

impl RammapApp {
    fn filtered_processes(&self) -> Vec<ProcessMemoryEntry> {
        let query = self.search_query.to_lowercase();
        self.processes
            .iter()
            .filter(|p| {
                query.is_empty() || {
                    let category = classify_process(&p.name);
                    p.name.to_lowercase().contains(&query)
                        || p.pid.to_string().contains(&query)
                        || category.label().to_lowercase().contains(&query)
                }
            })
            .cloned()
            .collect()
    }

    fn header<S: Fn(RammapMessage) + Clone + 'static>(&self, send: S) -> View {
        let title = TextBlock::new()
            .text("Nanai Lite Rammap (WizTree + GPU Edition)")
            .font_size(22.0)
            .font_weight(FontWeight::BOLD);
        let target_actions = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                TextBlock::new()
                    .text("Target:")
                    .font_size(12.0)
                    .vertical_alignment(VerticalAlignment::Center),
                self.selectable_button(
                    "💻 System RAM",
                    self.tab == MemoryTab::SystemRam,
                    RammapMessage::SetTab(MemoryTab::SystemRam),
                    send.clone(),
                ),
                self.selectable_button(
                    "💾 Virtual Mem",
                    self.tab == MemoryTab::VirtualMemory,
                    RammapMessage::SetTab(MemoryTab::VirtualMemory),
                    send.clone(),
                ),
                self.selectable_button(
                    "🎮 GPU VRAM",
                    self.tab == MemoryTab::GpuVram,
                    RammapMessage::SetTab(MemoryTab::GpuVram),
                    send.clone(),
                ),
                self.selectable_button(
                    "🧩 GPU Split",
                    self.tab == MemoryTab::Split,
                    RammapMessage::SetTab(MemoryTab::Split),
                    send.clone(),
                ),
                self.message_button("🔄 Refresh", RammapMessage::Refresh, send.clone()),
            ));
        let mut view_buttons: Vec<View> = vec![
            TextBlock::new()
                .text("View:")
                .font_size(12.0)
                .vertical_alignment(VerticalAlignment::Center)
                .into(),
            self.selectable_button(
                "📊 2D Tree",
                self.view_mode == ViewMode::Treemap,
                RammapMessage::SetViewMode(ViewMode::Treemap),
                send.clone(),
            ),
            self.selectable_button(
                "📋 List",
                self.view_mode == ViewMode::List,
                RammapMessage::SetViewMode(ViewMode::List),
                send.clone(),
            ),
            TextBlock::new()
                .text("Group:")
                .font_size(12.0)
                .vertical_alignment(VerticalAlignment::Center)
                .into(),
            self.selectable_button(
                "🗂️ Name",
                self.group_mode == GroupMode::ByName,
                RammapMessage::SetGroupMode(GroupMode::ByName),
                send.clone(),
            ),
            self.selectable_button(
                "🏷️ Category",
                self.group_mode == GroupMode::ByCategory,
                RammapMessage::SetGroupMode(GroupMode::ByCategory),
                send.clone(),
            ),
            self.selectable_button(
                "🔹 PID",
                self.group_mode == GroupMode::Individual,
                RammapMessage::SetGroupMode(GroupMode::Individual),
                send.clone(),
            ),
            self.message_button(self.metric_label(), RammapMessage::ToggleMetric, send.clone()),
            self.selectable_button(
                "🧱 Free",
                self.show_free_space,
                RammapMessage::ToggleFreeSpace,
                send.clone(),
            ),
        ];
        if self.tab == MemoryTab::GpuVram || self.tab == MemoryTab::Split {
            view_buttons.push(self.selectable_button(
                "⚖️ Scale",
                self.proportional_gpu,
                RammapMessage::ToggleProportionalGpu,
                send.clone(),
            ));
        }
        if self.tab == MemoryTab::GpuVram && self.gpu_adapters.len() > 1 {
            view_buttons.push(self.message_button(
                self.gpu_adapter_label(),
                RammapMessage::CycleGpuAdapter,
                send.clone(),
            ));
        }
        if self.tab == MemoryTab::Split {
            view_buttons.push(self.message_button(
                self.split_layout_label(),
                RammapMessage::ToggleSplitLayoutMode,
                send,
            ));
        }
        let view_actions = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children(view_buttons);
        StackPanel::new()
            .spacing(4.0)
            .children((
                title,
                TextBlock::new().text(self.subtitle()).font_size(12.0),
                target_actions,
                view_actions,
            ))
            .into()
    }

    fn subtitle(&self) -> String {
        let gpu = if self.gpu_adapters.is_empty() {
            "GPU: No DirectX 11/12 GPU detected".to_string()
        } else {
            self.gpu_adapters
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    let kind = if g.is_discrete { "dGPU" } else { "iGPU" };
                    format!(
                        "GPU {} [{}]: {} | Dedicated VRAM: {} / {} | Shared: {} / {}",
                        i,
                        kind,
                        g.name,
                        format_bytes(g.dedicated_usage_bytes),
                        format_bytes(g.dedicated_vram_bytes),
                        format_bytes(g.shared_usage_bytes),
                        format_bytes(g.shared_ram_bytes)
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ")
        };
        let ram_summary = format!(
            "Processes: {} | Physical RAM: {} (Used: {} / {}%) | Compressed: {} | Cache: {} | Paged: {} | NonPaged: {}",
            self.summary.process_count,
            format_bytes(self.summary.total_phys_bytes),
            format_bytes(self.summary.used_phys_bytes),
            self.summary.memory_load_pct,
            format_bytes(self.summary.compressed_bytes),
            format_bytes(self.summary.system_cache_bytes),
            format_bytes(self.summary.paged_pool_bytes),
            format_bytes(self.summary.nonpaged_pool_bytes)
        );
        let vmem_summary = format!(
            "Processes: {} | Commit: {} / {} | PageFile: {} / {}",
            self.summary.process_count,
            format_bytes(self.summary.commit_total_bytes),
            format_bytes(self.summary.commit_limit_bytes),
            format_bytes(
                self.summary
                    .total_page_file_bytes
                    .saturating_sub(self.summary.avail_page_file_bytes)
            ),
            format_bytes(self.summary.total_page_file_bytes)
        );
        let gpu_summary = format!("Processes: {} | {}", self.summary.process_count, gpu);
        match self.tab {
            MemoryTab::SystemRam => ram_summary,
            MemoryTab::VirtualMemory => vmem_summary,
            MemoryTab::GpuVram | MemoryTab::Split => gpu_summary,
        }
    }

    fn gpu_adapter_label(&self) -> String {
        if let Some(g) = self.gpu_adapters.get(self.selected_gpu_index) {
            let kind = if g.is_discrete { "dGPU" } else { "iGPU" };
            format!("🎮 GPU {} [{}]", self.selected_gpu_index, kind)
        } else {
            "🎮 GPU 0".to_string()
        }
    }

    fn split_layout_label(&self) -> &'static str {
        match self.split_layout_mode {
            SplitLayoutMode::Fit => "📐 Split: Fit",
            SplitLayoutMode::Scroll => "↔️ Split: Scroll",
        }
    }

    fn metric_label(&self) -> &'static str {
        if self.use_requested {
            "Metric: Private / Requested"
        } else {
            "Metric: Working Set / Usage"
        }
    }

    fn selectable_button<S: Fn(RammapMessage) + Clone + 'static>(
        &self,
        text: &'static str,
        selected: bool,
        message: RammapMessage,
        send: S,
    ) -> View {
        let mut btn = Button::new()
            .on_click(move || send(message.clone()))
            .content(TextBlock::new().text(text));
        if selected {
            btn = btn.background(Brush::Solid(Color::argb(255, 0, 120, 215)));
        }
        btn.into()
    }

    fn message_button<S: Fn(RammapMessage) + Clone + 'static>(
        &self,
        text: impl Into<String>,
        message: RammapMessage,
        send: S,
    ) -> View {
        let text = text.into();
        Button::new()
            .on_click(move || send(message.clone()))
            .content(TextBlock::new().text(text))
            .into()
    }

    fn search_panel<S: Fn(RammapMessage) + Clone + 'static>(&self, send: S) -> View {
        let selected = if let Some(ref title) = self.selected_group_title {
            format!("🎯 Selected Group: {}", title)
        } else if let Some(ref p) = self.selected_process {
            format!(
                "🎯 [{}] {} ({}) | WS: {} (Peak: {}) | Private: {} | Pagefile: {} | GPU VRAM: {} | Shared: {}",
                p.pid,
                p.name,
                classify_process(&p.name).label(),
                format_bytes(p.working_set_bytes),
                format_bytes(p.peak_working_set_bytes),
                format_bytes(p.private_bytes),
                format_bytes(p.pagefile_bytes),
                format_bytes(p.gpu_dedicated_bytes),
                format_bytes(p.gpu_shared_bytes)
            )
        } else {
            "💡 Click any 2D tile or table row to inspect details. Switch between System RAM and GPU VRAM tabs above.".to_string()
        };
        let search_send = send.clone();
        let search = TextBox::new()
            .placeholder_text("Filter PID, name or category...")
            .text(self.search_query.clone())
            .width(280.0)
            .on_text_changed(move |text: std::rc::Rc<str>| {
                search_send(RammapMessage::SearchChanged(text.to_string()))
            });
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                search,
                TextBlock::new()
                    .text(selected)
                    .font_size(12.0)
                    .vertical_alignment(VerticalAlignment::Center),
            ))
            .into()
    }
}
