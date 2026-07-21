//! Plugin system.
//!
//! A plugin turns the user's query into zero or more [`PluginResult`]s. This is
//! the primary extension point of the launcher: anything that reacts to what the
//! user types — a calculator, a unit converter, a web-search suggester — is a
//! [`Plugin`]. Built-in application and command listing predate this system and
//! still flow through [`crate::get_entities`], but they are expected to migrate
//! onto the same interface over time.

mod calculator;
mod loader;

pub use loader::plugins_dir;

use crate::common::Image;

/// Legacy static command entry. Predates [`Plugin`]; kept until commands are
/// reworked as plugin-provided results.
#[derive(Debug, Clone)]
pub struct CommandEntity {
    pub id: u64,
    pub name: String,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub image: Option<Image>,
    pub needs_argument: bool,
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
    /// Copy the given text to the system clipboard.
    CopyToClipboard(String),
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
    Grid { columns: u32, items: Vec<GridItem> },
    Detail { body: String, metadata: Vec<KeyValue> },
    Form { fields: Vec<FormField> },
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
    /// Search text changed (grid views).
    Search(String),
    /// A grid cell was activated, by its id.
    Activate(String),
    /// A form was submitted with the collected field values.
    Submit(Vec<FieldValue>),
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
    /// Perform an effect (copy, push another view, close).
    Effect(ActionEffect),
}

/// A query-driven source of results, optionally backed by interactive views.
pub trait Plugin: Send + Sync {
    /// Stable identifier for the plugin.
    fn id(&self) -> &str;
    /// Results for `query`. Return an empty vec when the query doesn't apply.
    fn query(&self, query: &str) -> Vec<PluginResult>;
    /// Handle an interaction within one of this plugin's views. Plugins that
    /// only produce list results can ignore this.
    fn handle_event(&self, _event: ViewEvent) -> ViewResponse {
        ViewResponse::None
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
            plugins: vec![Box::new(calculator::Calculator::new())],
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

    /// Collect results from every plugin for `query`, in registration order.
    pub fn query(&self, query: &str) -> Vec<PluginResult> {
        self.plugins
            .iter()
            .flat_map(|plugin| plugin.query(query))
            .collect()
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
