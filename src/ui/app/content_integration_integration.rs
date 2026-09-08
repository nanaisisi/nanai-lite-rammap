use super::content_integration::{grouped_list, individual_list};
use super::content_integration_core::{grouped_treemap, individual_treemap};
use crate::memory::ProcessMemoryEntry;
use crate::ui::app_core::RammapApp;
use crate::ui::types::{GroupMode, RammapMessage, ViewMode};
use windows_reactor::View;

pub(super) fn build<S>(app: &RammapApp, sender: S, filtered: Vec<ProcessMemoryEntry>) -> View
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
