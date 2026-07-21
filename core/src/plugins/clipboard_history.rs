//! Built-in clipboard history plugin.
//!
//! To capture *every* copy — not just whatever was on the clipboard when the
//! launcher opens — a background `wl-paste --watch <self> --clip-record` records
//! each change (`wl-paste --watch` pipes the new content to the command's stdin,
//! which the [`record`] entry point stores). That watcher is owned by the
//! resident agent (`ui`), which spawns it on start and stops it on quit, so
//! recording follows the agent's lifetime rather than lingering in the
//! background.
//!
//! The store file is the single source of truth, shared between the watcher and
//! the plugin. "Clipboard History" lists it (searchable); activating a row
//! copies that entry back to the clipboard.

use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use super::{
    ActionEffect, Command as PluginCommand, ListRow, Plugin, PluginMeta, Preference,
    PreferenceKind, PreferenceValue, View, ViewBody, ViewEvent, ViewEventKind, ViewResponse,
};
use crate::clipboard;
use crate::{APPLICATION, ORGANISATION, QUALIFIER};

const PLUGIN_ID: &str = "clipboard";
const VIEW_ID: &str = "clipboard-history";
const DEFAULT_MAX: usize = 100;
/// Options for the "History length" preference (paired with the labels below).
const LENGTHS: [usize; 4] = [50, 100, 500, 1000];

/// Records the text you copy so it can be searched and pasted again. Stateless:
/// the store file (shared with the background watcher) is the source of truth.
pub struct ClipboardHistory;

impl ClipboardHistory {
    pub fn new() -> Self {
        // The continuous background watcher is owned by the resident agent (so it
        // stops when the agent quits); here we just capture whatever is on the
        // clipboard right now.
        if let Some(text) = clipboard::paste() {
            record(&text);
        }
        ClipboardHistory
    }
}

impl Default for ClipboardHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ClipboardHistory {
    fn id(&self) -> &str {
        PLUGIN_ID
    }

    fn metadata(&self) -> PluginMeta {
        PluginMeta {
            name: Some("Clipboard History".to_string()),
            author: Some("built-in".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            description: Some(
                "Keep a searchable history of the text you copy and paste any past entry with \
                 a keystroke. A background watcher records every copy continuously."
                    .to_string(),
            ),
        }
    }

    fn preferences(&self) -> Vec<Preference> {
        let store = load();
        vec![
            Preference {
                id: "store_history".to_string(),
                label: "Store history".to_string(),
                hint: "Record clipboard contents in the background.".to_string(),
                kind: PreferenceKind::Toggle(store.enabled),
            },
            Preference {
                id: "history_length".to_string(),
                label: "History length".to_string(),
                hint: "How many entries to keep before old ones are pruned.".to_string(),
                kind: PreferenceKind::Select {
                    options: LENGTHS.iter().map(|len| format!("{len} entries")).collect(),
                    selected: LENGTHS
                        .iter()
                        .position(|len| *len == store.max)
                        .unwrap_or(1) as u64,
                },
            },
        ]
    }

    fn set_preference(&self, id: &str, value: PreferenceValue) {
        match (id, value) {
            ("store_history", PreferenceValue::Toggle(on)) => set_enabled(on),
            ("history_length", PreferenceValue::Choice(index)) => {
                set_max(LENGTHS.get(index as usize).copied().unwrap_or(DEFAULT_MAX));
            }
            _ => {}
        }
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            command(
                "history",
                "Clipboard History",
                "Browse and paste past copies",
                '⧉',
                &["clipboard", "clip", "history", "paste", "copy"],
            ),
            command(
                "clear",
                "Clear Clipboard History",
                "Delete all stored entries",
                '×',
                &["clipboard", "clear", "history"],
            ),
        ]
    }

    fn run_command(&self, command_id: &str, _argument: Option<&str>) -> ActionEffect {
        match command_id {
            "history" => ActionEffect::PushView(list_view(&load().entries)),
            "clear" => {
                clear();
                ActionEffect::Close
            }
            _ => ActionEffect::None,
        }
    }

    fn handle_event(&self, event: ViewEvent) -> ViewResponse {
        if event.view_id != VIEW_ID {
            return ViewResponse::None;
        }

        match event.kind {
            ViewEventKind::Search(term) => {
                let needle = term.trim().to_lowercase();
                let entries: Vec<String> = load()
                    .entries
                    .into_iter()
                    .filter(|entry| needle.is_empty() || entry.to_lowercase().contains(&needle))
                    .collect();
                ViewResponse::Update(list_view(&entries))
            }
            // The row id is the entry's full text; copy it back to the clipboard.
            ViewEventKind::Activate(content) => {
                ViewResponse::Effect(ActionEffect::CopyToClipboard(content))
            }
            _ => ViewResponse::None,
        }
    }
}

fn command(id: &str, title: &str, subtitle: &str, glyph: char, keywords: &[&str]) -> PluginCommand {
    PluginCommand {
        id: id.to_string(),
        title: title.to_string(),
        subtitle: Some(subtitle.to_string()),
        keywords: keywords.iter().map(|k| k.to_string()).collect(),
        icon: None,
        glyph: Some(glyph),
        category: "Clipboard".to_string(),
        needs_argument: false,
        argument_placeholder: None,
        fallback: false,
    }
}

/// Build the searchable list view from `entries`.
fn list_view(entries: &[String]) -> View {
    View {
        view_id: VIEW_ID.to_string(),
        title: "Clipboard History".to_string(),
        search_placeholder: Some("Search clipboard…".to_string()),
        submit_label: Some("Paste".to_string()),
        body: ViewBody::List {
            items: entries
                .iter()
                .map(|content| ListRow {
                    id: content.clone(),
                    title: preview(content),
                    subtitle: Some(meta_line(content)),
                    glyph: Some('⧉'),
                })
                .collect(),
        },
    }
}

/// A one-line preview: the first non-blank line, trimmed and clipped, with an
/// ellipsis when there is more.
fn preview(content: &str) -> String {
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    let first = lines.first().map(|l| l.trim()).unwrap_or("");
    let mut preview: String = first.chars().take(80).collect();
    if first.chars().count() > 80 || lines.len() > 1 {
        preview.push('…');
    }
    if preview.is_empty() {
        preview.push_str("(whitespace)");
    }
    preview
}

/// The secondary line: character count, plus line count when multi-line.
fn meta_line(content: &str) -> String {
    let chars = content.chars().count();
    let lines = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count()
        .max(1);
    if lines > 1 {
        format!("{chars} characters · {lines} lines")
    } else {
        format!("{chars} characters")
    }
}

// --- Persistence ------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct Store {
    #[serde(default)]
    entries: Vec<String>,
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default = "default_max")]
    max: usize,
}

impl Default for Store {
    fn default() -> Self {
        Store {
            entries: Vec::new(),
            enabled: true,
            max: DEFAULT_MAX,
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_max() -> usize {
    DEFAULT_MAX
}

fn store_path() -> Option<PathBuf> {
    ProjectDirs::from(QUALIFIER, ORGANISATION, APPLICATION)
        .map(|dirs| dirs.data_local_dir().join("clipboard_history.toml"))
}

fn load_at(path: &Path) -> Store {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_at(path: &Path, store: &Store) {
    if let Ok(text) = toml::to_string(store) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, text);
    }
}

/// Prepend `content` as the newest entry (de-duplicating and pruning), honoring
/// the "Store history" setting. Reloads before writing so it never clobbers
/// entries the watcher recorded meanwhile.
fn record_at(path: &Path, content: &str) {
    if content.trim().is_empty() {
        return;
    }
    let mut store = load_at(path);
    if !store.enabled {
        return;
    }
    if store.entries.first().is_some_and(|first| first == content) {
        return; // already the newest entry
    }
    store.entries.retain(|entry| entry != content);
    store.entries.insert(0, content.to_string());
    store.entries.truncate(store.max.max(1));
    save_at(path, &store);
}

fn load() -> Store {
    store_path().map(|path| load_at(&path)).unwrap_or_default()
}

/// Record a clipboard entry into the store. Called both on launcher open and by
/// the background `--clip-record` watcher invocation.
pub fn record(content: &str) {
    if let Some(path) = store_path() {
        record_at(&path, content);
    }
}

/// Whether clipboard capture is currently enabled (the "Store history" pref).
pub fn recording_enabled() -> bool {
    load().enabled
}

/// Enable or disable clipboard capture (mirrors the "Store history" pref).
pub fn set_recording(enabled: bool) {
    set_enabled(enabled);
}

/// Empty the clipboard history.
pub fn clear_history() {
    clear();
}

fn clear() {
    if let Some(path) = store_path() {
        let mut store = load_at(&path);
        store.entries.clear();
        save_at(&path, &store);
    }
}

fn set_enabled(on: bool) {
    if let Some(path) = store_path() {
        let mut store = load_at(&path);
        store.enabled = on;
        save_at(&path, &store);
    }
}

fn set_max(max: usize) {
    if let Some(path) = store_path() {
        let mut store = load_at(&path);
        store.max = max.max(1);
        store.entries.truncate(store.max);
        save_at(&path, &store);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("iced_raycast_clip_{name}.toml"));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn preview_and_meta_line() {
        assert_eq!(preview("hello world"), "hello world");
        // Multi-line collapses to the first non-blank line with an ellipsis.
        assert_eq!(preview("  line one\nline two  "), "line one…");
        assert_eq!(preview("   "), "(whitespace)");
        assert_eq!(meta_line("abcd"), "4 characters");
        assert!(meta_line("a\nb").contains("2 lines"));
    }

    #[test]
    fn record_prepends_and_deduplicates() {
        let path = temp_store("dedup");
        record_at(&path, "one");
        record_at(&path, "two");
        record_at(&path, "one"); // moves "one" back to the front, no duplicate
        assert_eq!(load_at(&path).entries, vec!["one", "two"]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn record_respects_disabled_and_prunes_to_max() {
        let path = temp_store("prune");
        save_at(
            &path,
            &Store {
                entries: vec![],
                enabled: true,
                max: 2,
            },
        );
        record_at(&path, "a");
        record_at(&path, "b");
        record_at(&path, "c"); // over max → oldest ("a") pruned
        assert_eq!(load_at(&path).entries, vec!["c", "b"]);

        // Disabled: new content is ignored.
        save_at(
            &path,
            &Store {
                entries: vec!["c".into(), "b".into()],
                enabled: false,
                max: 2,
            },
        );
        record_at(&path, "d");
        assert_eq!(load_at(&path).entries, vec!["c", "b"]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn activating_a_row_copies_its_content() {
        let plugin = ClipboardHistory;
        match plugin.handle_event(ViewEvent {
            view_id: VIEW_ID.to_string(),
            kind: ViewEventKind::Activate("copied text".to_string()),
        }) {
            ViewResponse::Effect(ActionEffect::CopyToClipboard(text)) => {
                assert_eq!(text, "copied text")
            }
            other => panic!("expected a copy effect, got {other:?}"),
        }
    }
}
