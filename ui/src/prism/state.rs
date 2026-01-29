use std::collections::HashMap;

use crate::prism::items::ListEntry;
use core::AppState;
use iced::widget::Id;

#[derive(Clone, Debug)]
pub struct PrismEntry {
    pub entry: ListEntry,
    pub id: Id,
}

impl From<ListEntry> for PrismEntry {
    fn from(entry: ListEntry) -> Self {
        Self {
            entry,
            id: Id::unique(),
        }
    }
}

pub struct PrismState {
    pub query: String,
    pub all_entries: Vec<PrismEntry>,
    pub entries: Vec<PrismEntry>,
    pub selected_index: usize,
    pub search_id: Id,
    pub scroll_id: Id,
    pub viewport_height: f32,
    pub current_scroll_offset: f32,
    pub height_cache: HashMap<Id, f32>,
    pub default_row_height: f32,
    pub app_state: AppState,
}
