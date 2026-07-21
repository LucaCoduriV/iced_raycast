use std::collections::HashMap;

use crate::prism::items::ListEntry;
use core::{FieldKind, FieldValueKind, View, ViewBody};
use iced::widget::Id;

#[derive(Clone, Debug)]
pub struct PrismEntry {
    pub entry: ListEntry,
    pub id: Id,
    /// Lowercased "name\ndescription", precomputed once so the per-keystroke
    /// filter is a plain substring check with no allocation.
    pub search_haystack: String,
}

impl From<ListEntry> for PrismEntry {
    fn from(entry: ListEntry) -> Self {
        let mut search_haystack = entry.name().to_lowercase();
        if let Some(desc) = entry.description() {
            search_haystack.push('\n');
            search_haystack.push_str(&desc.to_lowercase());
        }

        Self {
            entry,
            id: Id::unique(),
            search_haystack,
        }
    }
}

pub struct PrismState {
    pub query: String,
    pub argument: Option<String>,
    pub all_entries: Vec<PrismEntry>,
    pub entries: Vec<PrismEntry>,
    pub selected_index: usize,
    pub search_id: Id,
    pub argument_id: Id,
    pub scroll_id: Id,
    pub viewport_height: f32,
    pub current_scroll_offset: f32,
    pub height_cache: HashMap<Id, f32>,
    pub default_row_height: f32,
    pub show_argument_input: bool,
    pub is_argument_input_active: bool,
    pub show_actions: bool,
    pub actions_selected_index: usize,
    /// Recent arguments for the command currently in argument mode.
    pub recent_arguments: Vec<String>,
    /// Navigation stack of plugin views. Empty means the normal list; the last
    /// element is the active full-screen view.
    pub views: Vec<ViewState>,
}

/// Host-held interaction state for one plugin view on the navigation stack.
pub struct ViewState {
    /// Plugin that owns this view (events route back to it).
    pub plugin_id: String,
    /// The current view contents (replaced when the plugin sends an update).
    pub view: View,
    /// In-view search text.
    pub search: String,
    /// Selected grid cell index.
    pub selected: usize,
    /// Current values of form fields, keyed by field id.
    pub form_values: HashMap<String, FieldValueKind>,
    pub search_id: Id,
}

impl ViewState {
    pub fn new(plugin_id: String, view: View) -> Self {
        // Seed form state from each field's initial value.
        let mut form_values = HashMap::new();
        if let ViewBody::Form { fields } = &view.body {
            for field in fields {
                let value = match &field.kind {
                    FieldKind::Text(v) | FieldKind::TextArea(v) => FieldValueKind::Text(v.clone()),
                    FieldKind::Toggle(v) => FieldValueKind::Toggle(*v),
                    FieldKind::Dropdown { selected, .. } => FieldValueKind::Choice(*selected),
                };
                form_values.insert(field.id.clone(), value);
            }
        }

        Self {
            plugin_id,
            view,
            search: String::new(),
            selected: 0,
            form_values,
            search_id: Id::unique(),
        }
    }

    /// Collect current form values in field order.
    pub fn collect_form_values(&self) -> Vec<core::FieldValue> {
        match &self.view.body {
            ViewBody::Form { fields } => fields
                .iter()
                .filter_map(|field| {
                    self.form_values.get(&field.id).map(|value| core::FieldValue {
                        id: field.id.clone(),
                        value: value.clone(),
                    })
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Number of selectable grid cells (0 for non-grid bodies).
    pub fn grid_len(&self) -> usize {
        match &self.view.body {
            ViewBody::Grid { items, .. } => items.len(),
            _ => 0,
        }
    }

    /// Column count of the active grid (at least 1; 1 for non-grid bodies).
    pub fn grid_columns(&self) -> usize {
        match &self.view.body {
            ViewBody::Grid { columns, .. } => (*columns as usize).max(1),
            _ => 1,
        }
    }
}
