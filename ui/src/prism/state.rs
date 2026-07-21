use std::collections::HashMap;

use crate::prism::items::ListEntry;
use core::{FieldKind, FieldValueKind, View, ViewBody};
use iced::widget::{Id, image};

/// Host-side cache of fetched grid images, keyed by URL. Handles are created
/// once (stable id) so re-renders reuse the GPU texture instead of reloading.
pub type ImageCache = HashMap<String, ImageEntry>;

#[derive(Clone, Debug)]
pub enum ImageEntry {
    Loading,
    /// Decoded frames as `(handle, delay_ms)`. A single frame is a still image;
    /// the handles are built once so re-renders reuse the GPU textures.
    Loaded(Vec<(image::Handle, u32)>),
    Failed,
}

impl ImageEntry {
    /// The frame to display for the given elapsed animation time.
    pub fn frame_at(&self, elapsed_ms: u64) -> Option<&image::Handle> {
        let Self::Loaded(frames) = self else {
            return None;
        };
        match frames.as_slice() {
            [] => None,
            [(handle, _)] => Some(handle),
            frames => {
                let total: u64 = frames.iter().map(|(_, delay)| *delay as u64).sum();
                if total == 0 {
                    return Some(&frames[0].0);
                }
                let mut t = elapsed_ms % total;
                for (handle, delay) in frames {
                    let delay = *delay as u64;
                    if t < delay {
                        return Some(handle);
                    }
                    t -= delay;
                }
                frames.last().map(|(handle, _)| handle)
            }
        }
    }

    /// Whether this is a multi-frame animation.
    pub fn is_animated(&self) -> bool {
        matches!(self, Self::Loaded(frames) if frames.len() > 1)
    }
}

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
    /// Fetched grid images, shared across views.
    pub image_cache: ImageCache,
    /// Animation clock: `now - epoch` drives GIF frame selection.
    pub anim_epoch: std::time::Instant,
    pub anim_now: std::time::Instant,
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
    /// True while a search request is in flight (drives the loading indicator).
    pub searching: bool,
    /// Latest search term typed while a request was already in flight; run once
    /// the current one returns (coalesces rapid typing).
    pub pending_search: Option<String>,
    pub scroll_id: Id,
    /// Number of grid items currently loaded (the next page's offset).
    pub loaded_count: usize,
    /// True while a "load more" (pagination) request is in flight.
    pub loading_more: bool,
    /// False once a page comes back empty — no more results to fetch.
    pub more_available: bool,
    /// Last-known scroll offset and viewport height, for keyboard scroll-follow.
    pub scroll_offset: f32,
    pub viewport_height: f32,
    /// One id per grid row (for measuring actual row heights).
    pub row_ids: Vec<Id>,
    /// Measured height of each grid row, keyed by its id.
    pub row_heights: HashMap<Id, f32>,
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
            searching: false,
            pending_search: None,
            scroll_id: Id::unique(),
            loaded_count: 0,
            loading_more: false,
            more_available: true,
            scroll_offset: 0.0,
            viewport_height: 0.0,
            row_ids: Vec::new(),
            row_heights: HashMap::new(),
        }
    }

    /// Collect current form values in field order.
    pub fn collect_form_values(&self) -> Vec<core::FieldValue> {
        match &self.view.body {
            ViewBody::Form { fields } => fields
                .iter()
                .filter_map(|field| {
                    self.form_values
                        .get(&field.id)
                        .map(|value| core::FieldValue {
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
