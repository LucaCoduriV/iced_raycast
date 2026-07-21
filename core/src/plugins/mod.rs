//! Plugin system.
//!
//! A plugin turns the user's query into zero or more [`PluginResult`]s. This is
//! the primary extension point of the launcher: anything that reacts to what the
//! user types — a calculator, a unit converter, a web-search suggester — is a
//! [`Plugin`]. Built-in application and command listing predate this system and
//! still flow through [`crate::get_entities`], but they are expected to migrate
//! onto the same interface over time.

mod calculator;

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
}

/// A query-driven source of results.
pub trait Plugin: Send + Sync {
    /// Stable identifier for the plugin.
    fn id(&self) -> &str;
    /// Results for `query`. Return an empty vec when the query doesn't apply.
    fn query(&self, query: &str) -> Vec<PluginResult>;
}

/// Holds the active plugins and fans a query out to all of them.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    /// Registry populated with the built-in plugins.
    pub fn with_builtins() -> Self {
        Self {
            plugins: vec![Box::new(calculator::Calculator::new())],
        }
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
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}
