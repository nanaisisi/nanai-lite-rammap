#[path = "app/content_core.rs"]
mod content_core;
#[path = "app/content_integration.rs"]
mod content_integration;

use crate::category::classify_process;
use crate::memory::{format_bytes, ProcessMemoryEntry};
use crate::ui::types::{AppInput, GroupMode, MemoryMetric, MemoryTab, RammapMessage, ViewMode};
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
                sender.send(message);
            }
        };
        let content = content_integration::build(self, send.clone(), self.filtered_processes());
        StackPanel::new()
            .spacing(10.0)
            .margin(Thickness::uniform(14.0))
            .children((self.header(send.clone()), self.search_panel(send), content))
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
        let actions = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                self.tab_button("💻 System RAM", MemoryTab::SystemRam, send.clone()),
                self.tab_button("🎮 GPU VRAM", MemoryTab::GpuVram, send.clone()),
                self.message_button("🔄 Refresh", RammapMessage::Refresh, send.clone()),
                self.mode_button("📊 2D Tree", ViewMode::Treemap, send.clone()),
                self.mode_button("📋 List", ViewMode::List, send.clone()),
                self.group_button("🗂️ By Name", GroupMode::ByName, send.clone()),
                self.group_button("🏷️ By Category", GroupMode::ByCategory, send.clone()),
                self.group_button("🔹 Individual", GroupMode::Individual, send.clone()),
                self.message_button(self.metric_label(), RammapMessage::ToggleMetric, send),
            ));
        StackPanel::new().spacing(4.0).children((
            title,
            TextBlock::new().text(self.subtitle()).font_size(12.0),
            actions,
        ))
    }

    fn subtitle(&self) -> String {
        let gpu = if self.gpu_adapters.is_empty() {
            "GPU: No DirectX 11/12 GPU detected".to_string()
        } else {
            self.gpu_adapters
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    format!(
                        "GPU {}: {} | Dedicated VRAM: {} / {} | Shared: {} / {}",
                        i,
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
        match self.tab {
            MemoryTab::SystemRam => format!(
                "Processes: {} | Physical RAM: {} (Used: {} / {}%) | Commit: {} / {}",
                self.summary.process_count,
                format_bytes(self.summary.total_phys_bytes),
                format_bytes(self.summary.used_phys_bytes),
                self.summary.memory_load_pct,
                format_bytes(self.summary.commit_total_bytes),
                format_bytes(self.summary.commit_limit_bytes)
            ),
            MemoryTab::GpuVram => format!("Processes: {} | {}", self.summary.process_count, gpu),
        }
    }

    fn metric_label(&self) -> &'static str {
        match self.metric {
            MemoryMetric::WorkingSet => "Metric: Working Set",
            MemoryMetric::PrivateWs => "Metric: Private (Commit)",
            MemoryMetric::GpuDedicated => "Metric: Dedicated VRAM",
            MemoryMetric::GpuShared => "Metric: Shared GPU Memory",
        }
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
    }

    fn tab_button<S: Fn(RammapMessage) + Clone + 'static>(
        &self,
        text: &'static str,
        tab: MemoryTab,
        send: S,
    ) -> View {
        self.message_button(text, RammapMessage::SetTab(tab), send)
    }
    fn mode_button<S: Fn(RammapMessage) + Clone + 'static>(
        &self,
        text: &'static str,
        mode: ViewMode,
        send: S,
    ) -> View {
        self.message_button(text, RammapMessage::SetViewMode(mode), send)
    }
    fn group_button<S: Fn(RammapMessage) + Clone + 'static>(
        &self,
        text: &'static str,
        mode: GroupMode,
        send: S,
    ) -> View {
        self.message_button(text, RammapMessage::SetGroupMode(mode), send)
    }

    fn search_panel<S: Fn(RammapMessage) + Clone + 'static>(&self, send: S) -> View {
        let selected = if let Some(ref title) = self.selected_group_title {
            format!("🎯 Selected Group: {}", title)
        } else if let Some(ref p) = self.selected_process {
            format!(
                "🎯 [{}] {} ({}) | WS: {} | Private: {} | GPU VRAM: {} | GPU Shared: {}",
                p.pid,
                p.name,
                classify_process(&p.name).label(),
                format_bytes(p.working_set_bytes),
                format_bytes(p.private_bytes),
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
            .on_text_changed(move |text: String| search_send(RammapMessage::SearchChanged(text)));
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
    }
}
