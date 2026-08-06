//! Plugin system.
//!
//! A plugin turns the user's query into zero or more [`PluginResult`]s. This is
//! the primary extension point of the launcher: anything that reacts to what the
//! user types — a calculator, a unit converter, a web-search suggester — is a
//! [`Plugin`]. Built-in application and command listing predate this system and
//! still flow through [`crate::get_entities`], but they are expected to migrate
//! onto the same interface over time.

mod calculator;
mod clipboard_history;
mod loader;

pub use clipboard_history::{
    clear_history as clear_clipboard_history, record as record_clipboard,
    recording_enabled as clipboard_recording_enabled, set_recording as set_clipboard_recording,
};
pub use loader::plugins_dir;

use crate::common::Image;

/// A statically-registered command a plugin contributes to the main list.
/// Listed and searchable by title + keywords (unlike per-query results).
#[derive(Debug, Clone)]
pub struct Command {
    /// Stable id, passed back to [`Plugin::run_command`] on activation.
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    /// Extra search terms matched alongside the title.
    pub keywords: Vec<String>,
    pub icon: Option<Image>,
    pub glyph: Option<char>,
    /// Right-hand category label shown in the list.
    pub category: String,
    pub needs_argument: bool,
    pub argument_placeholder: Option<String>,
    /// Offered at the bottom of the list (using the typed query as its argument)
    /// so it can always be run on whatever the user typed.
    pub fallback: bool,
}

/// A single result surfaced by a [`Plugin`] for the current query.
#[derive(Debug, Clone)]
pub struct PluginResult {
    /// Id of the plugin that produced this result.
    pub source_id: String,
    /// Section this result is grouped under in the list (e.g. "Calculator").
    pub section: String,
    /// Primary line (e.g. the computed value).
    pub title: String,
    /// Optional secondary line (e.g. the normalized expression).
    pub subtitle: Option<String>,
    /// Optional real icon. When absent the UI renders a letter/glyph tile.
    pub icon: Option<Image>,
    /// Single-character glyph for the fallback tile (e.g. '=' for math).
    pub glyph: Option<char>,
    /// Available actions, most important first. The first is the default
    /// (triggered by Enter); the rest appear in the actions menu.
    pub actions: Vec<PluginAction>,
}

/// One entry in a result's action list.
#[derive(Debug, Clone)]
pub struct PluginAction {
    pub label: String,
    pub effect: ActionEffect,
}

/// A side effect the host performs when an action is invoked. Kept as data
/// (rather than a callback) so effects can cross the core/UI boundary and be
/// executed with the right framework primitives (e.g. clipboard access).
#[derive(Debug, Clone)]
pub enum ActionEffect {
    /// Do nothing.
    None,
    /// Copy the given text to the system clipboard.
    CopyToClipboard(String),
    /// Download the image at `url` (off the UI thread) and copy its bytes to
    /// the system clipboard under the given MIME type (e.g. `"image/gif"`).
    CopyImageFromUrl { url: String, mime: String },
    /// Open a URL in the user's default browser.
    OpenUrl(String),
    /// Push a full-screen plugin view onto the navigation stack.
    PushView(View),
    /// Close the launcher.
    Close,
}

// ---------------------------------------------------------------------------
// Views — full-screen layouts a plugin can push (grid / detail / form).
// ---------------------------------------------------------------------------

/// A full-screen view a plugin pushes onto the navigation stack.
#[derive(Debug, Clone)]
pub struct View {
    /// Stable id used to route events back to the plugin's view.
    pub view_id: String,
    /// Header title.
    pub title: String,
    /// If present, a search bar is shown and typing emits [`ViewEventKind::Search`].
    pub search_placeholder: Option<String>,
    /// Footer primary-action label (e.g. "Copy Image", "Create Snippet").
    pub submit_label: Option<String>,
    pub body: ViewBody,
}

/// The body of a [`View`] — one of the supported layouts.
#[derive(Debug, Clone)]
pub enum ViewBody {
    Grid {
        columns: u32,
        items: Vec<GridItem>,
    },
    /// A vertical list of text rows (e.g. clipboard history, snippets).
    List {
        items: Vec<ListRow>,
    },
    Detail {
        body: String,
        metadata: Vec<KeyValue>,
    },
    Form {
        fields: Vec<FormField>,
    },
}

/// A row in a [`ViewBody::List`].
#[derive(Debug, Clone)]
pub struct ListRow {
    /// Stable id echoed back to the plugin when this row is activated.
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    /// Optional single-character glyph for the row's leading tile.
    pub glyph: Option<char>,
}

/// A cell in a grid view.
#[derive(Debug, Clone)]
pub struct GridItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub image: ImageSource,
}

/// Where a grid cell's image comes from.
#[derive(Debug, Clone)]
pub enum ImageSource {
    None,
    Path(String),
    Bytes(Vec<u8>),
    /// A remote URL the host fetches (and caches) asynchronously.
    Url(String),
}

/// A key/value row in a detail view's metadata sidebar.
#[derive(Debug, Clone)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

/// A field in a form view.
#[derive(Debug, Clone)]
pub struct FormField {
    pub id: String,
    pub label: String,
    pub kind: FieldKind,
}

/// The kind (and initial value) of a form field.
#[derive(Debug, Clone)]
pub enum FieldKind {
    Text(String),
    TextArea(String),
    Toggle(bool),
    Dropdown { options: Vec<String>, selected: u64 },
}

/// An interaction reported to the plugin for the active view.
#[derive(Debug, Clone)]
pub struct ViewEvent {
    pub view_id: String,
    pub kind: ViewEventKind,
}

/// The kind of view interaction.
#[derive(Debug, Clone)]
pub enum ViewEventKind {
    /// Search text changed (grid views). Resets to the first page.
    Search(String),
    /// A grid cell was activated, by its id.
    Activate(String),
    /// A form was submitted with the collected field values.
    Submit(Vec<FieldValue>),
    /// Fetch the next page for `term` starting at `offset` items.
    LoadMore { term: String, offset: u64 },
}

/// A value collected from a form field on submit.
#[derive(Debug, Clone)]
pub struct FieldValue {
    pub id: String,
    pub value: FieldValueKind,
}

/// The concrete value of a submitted form field.
#[derive(Debug, Clone)]
pub enum FieldValueKind {
    Text(String),
    Toggle(bool),
    Choice(u64),
}

/// How the plugin responds to a view event.
#[derive(Debug, Clone)]
pub enum ViewResponse {
    None,
    /// Replace the current view's contents (e.g. new search results).
    Update(View),
    /// Append grid cells to the current view (pagination). Empty means no more.
    Append(Vec<GridItem>),
    /// Perform an effect (copy, push another view, close).
    Effect(ActionEffect),
}

/// Descriptive metadata a plugin declares about itself, shown in the Plugin
/// Manager. Absent fields fall back to host-derived defaults.
#[derive(Debug, Clone, Default)]
pub struct PluginMeta {
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

/// A user-facing preference a plugin exposes in its settings section.
#[derive(Debug, Clone)]
pub struct Preference {
    pub id: String,
    pub label: String,
    pub hint: String,
    pub kind: PreferenceKind,
}

/// The control (and current value) of a [`Preference`].
#[derive(Debug, Clone)]
pub enum PreferenceKind {
    Toggle(bool),
    Select { options: Vec<String>, selected: u64 },
    Text(String),
    Secret(String),
}

/// A concrete preference value: the persisted state of a [`Preference`], and
/// what is pushed to a plugin via [`Plugin::set_preference`]. Serialized
/// untagged (a bare bool / integer / string) so it stays TOML-friendly.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum PreferenceValue {
    Toggle(bool),
    Choice(u64),
    Text(String),
}

/// Install-time facts about a dynamically-loaded plugin's library file. Not
/// declared by the plugin — derived host-side from the `.so`/`.dll`/`.dylib`.
#[derive(Debug, Clone)]
pub struct InstallInfo {
    pub size_bytes: u64,
    pub modified: Option<std::time::SystemTime>,
}

/// A source of launcher functionality: static commands, live query results, and
/// interactive views.
pub trait Plugin: Send + Sync {
    /// Stable identifier for the plugin.
    fn id(&self) -> &str;

    /// Statically-registered commands, listed and searchable in the main list.
    fn commands(&self) -> Vec<Command> {
        Vec::new()
    }

    /// Activate a command (optionally with an argument) and return its effect.
    fn run_command(&self, _command_id: &str, _argument: Option<&str>) -> ActionEffect {
        ActionEffect::None
    }

    /// Live, per-keystroke results (e.g. a calculator).
    fn query(&self, _query: &str) -> Vec<PluginResult> {
        Vec::new()
    }

    /// Handle an interaction within one of this plugin's views.
    fn handle_event(&self, _event: ViewEvent) -> ViewResponse {
        ViewResponse::None
    }

    /// Descriptive metadata shown in the Plugin Manager.
    fn metadata(&self) -> PluginMeta {
        PluginMeta::default()
    }

    /// User-facing preferences shown in the plugin's settings section.
    fn preferences(&self) -> Vec<Preference> {
        Vec::new()
    }

    /// Notify the plugin that preference `id` changed to `value` (also called on
    /// startup to rehydrate persisted values).
    fn set_preference(&self, _id: &str, _value: PreferenceValue) {}

    /// Install-time facts about this plugin's library file, if it is a
    /// dynamically-loaded plugin (built-ins return `None`).
    fn install_info(&self) -> Option<InstallInfo> {
        None
    }
}

/// Holds the active plugins and fans a query out to all of them.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    /// Registry populated with the built-in plugins plus any compiled plugins
    /// installed in the [`plugins_dir`].
    pub fn with_builtins() -> Self {
        let mut registry = Self {
            plugins: vec![
                Box::new(calculator::Calculator::new()),
                Box::new(clipboard_history::ClipboardHistory::new()),
            ],
        };

        if let Some(dir) = plugins_dir() {
            for plugin in loader::load_plugins_from_dir(&dir) {
                registry.plugins.push(plugin);
            }
        }

        registry
    }

    /// Register an additional plugin.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// The id of every registered plugin, in registration order. Includes
    /// query-only plugins (e.g. the calculator) that contribute no commands.
    pub fn plugin_ids(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.id().to_string()).collect()
    }

    /// Every static command from every plugin, tagged with its plugin id.
    pub fn commands(&self) -> Vec<(String, Command)> {
        self.plugins
            .iter()
            .flat_map(|plugin| {
                let plugin_id = plugin.id().to_string();
                plugin
                    .commands()
                    .into_iter()
                    .map(move |command| (plugin_id.clone(), command))
            })
            .collect()
    }

    /// Activate a command on its owning plugin.
    pub fn run_command(
        &self,
        plugin_id: &str,
        command_id: &str,
        argument: Option<&str>,
    ) -> ActionEffect {
        self.plugins
            .iter()
            .find(|plugin| plugin.id() == plugin_id)
            .map(|plugin| plugin.run_command(command_id, argument))
            .unwrap_or(ActionEffect::None)
    }

    /// Collect live results from every plugin for `query`, in registration order.
    pub fn query(&self, query: &str) -> Vec<PluginResult> {
        self.plugins
            .iter()
            .flat_map(|plugin| plugin.query(query))
            .collect()
    }

    /// A plugin's self-declared metadata, by id.
    pub fn metadata(&self, plugin_id: &str) -> Option<PluginMeta> {
        self.plugins
            .iter()
            .find(|plugin| plugin.id() == plugin_id)
            .map(|plugin| plugin.metadata())
    }

    /// A plugin's user-facing preferences, by id (empty if unknown).
    pub fn preferences(&self, plugin_id: &str) -> Vec<Preference> {
        self.plugins
            .iter()
            .find(|plugin| plugin.id() == plugin_id)
            .map(|plugin| plugin.preferences())
            .unwrap_or_default()
    }

    /// Push a changed (or rehydrated) preference value to its owning plugin.
    pub fn set_preference(&self, plugin_id: &str, pref_id: &str, value: PreferenceValue) {
        if let Some(plugin) = self.plugins.iter().find(|plugin| plugin.id() == plugin_id) {
            plugin.set_preference(pref_id, value);
        }
    }

    /// Install-time facts about a plugin's library file, by id (`None` for
    /// built-ins or unknown plugins).
    pub fn install_info(&self, plugin_id: &str) -> Option<InstallInfo> {
        self.plugins
            .iter()
            .find(|plugin| plugin.id() == plugin_id)
            .and_then(|plugin| plugin.install_info())
    }

    /// Route a view event to its owning plugin (by id) and return the response.
    pub fn handle_event(&self, plugin_id: &str, event: ViewEvent) -> ViewResponse {
        self.plugins
            .iter()
            .find(|plugin| plugin.id() == plugin_id)
            .map(|plugin| plugin.handle_event(event))
            .unwrap_or(ViewResponse::None)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}
