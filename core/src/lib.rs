use crate::plugins::Command;

pub use crate::common::Image;
pub use crate::plugins::{
    ActionEffect, Command as PluginCommand, FieldKind, FieldValue, FieldValueKind, FormField,
    GridItem, ImageSource, KeyValue, Plugin, PluginAction, PluginRegistry, PluginResult, View,
    ViewBody, ViewEvent, ViewEventKind, ViewResponse,
};
use anyhow::Result;
pub use application::App;
pub use application::Application;
pub use common::AppState;

mod application;
pub mod clipboard;
mod common;
pub mod media;
pub mod net;
pub mod open;
mod plugins;
pub mod search;

const QUALIFIER: &str = "com";
const ORGANISATION: &str = "lcvitor";
const APPLICATION: &str = "iced_raycast";

/// A single listed, activatable item. Applications and plugin-provided commands
/// are the searchable entries; `Plugin` holds a live per-query result.
#[derive(Debug, Clone)]
pub enum Entity {
    Application(App),
    /// A static command contributed by a plugin. `fallback_query` is set when
    /// this is a fallback offered for the current (non-matching) query.
    Command {
        plugin_id: String,
        command: Command,
        fallback_query: Option<String>,
    },
    /// A dynamic result produced by a [`Plugin`] for the current query.
    Plugin(PluginResult),
}

impl Entity {
    pub fn name(&self) -> &str {
        match self {
            Entity::Application(app) => app.name(),
            Entity::Command { command, .. } => &command.title,
            Entity::Plugin(result) => &result.title,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            Entity::Application(app) => app.description(),
            // A fallback shows the query it will run on.
            Entity::Command {
                fallback_query: Some(query),
                ..
            } => Some(query),
            Entity::Command { command, .. } => command.subtitle.as_deref(),
            Entity::Plugin(result) => result.subtitle.as_deref(),
        }
    }

    /// Extra search terms, matched alongside the name/description.
    pub fn keywords(&self) -> &[String] {
        match self {
            Entity::Command { command, .. } => &command.keywords,
            _ => &[],
        }
    }

    pub fn icon(&self) -> Option<Image> {
        match self {
            Entity::Application(app) => app.icon(),
            Entity::Command { command, .. } => command.icon.clone(),
            Entity::Plugin(result) => result.icon.clone(),
        }
    }

    /// Launch a system application. Commands and plugin results act through
    /// effects instead (see [`Entity::command_ref`] / [`Entity::primary_effect`]).
    pub fn execute(&self, argument: Option<String>) -> Result<()> {
        match self {
            Entity::Application(app) => app.execute(argument),
            _ => Ok(()),
        }
    }

    pub fn needs_argument(&self) -> bool {
        match self {
            // A fallback already has its argument (the typed query).
            Entity::Command {
                fallback_query: Some(_),
                ..
            } => false,
            Entity::Command { command, .. } => command.needs_argument,
            _ => false,
        }
    }

    /// Placeholder text for the argument input, if this entity takes one.
    pub fn argument_placeholder(&self) -> Option<&str> {
        match self {
            Entity::Command { command, .. } => command.argument_placeholder.as_deref(),
            _ => None,
        }
    }

    /// Singular label shown on the right of a result row.
    pub fn kind_label(&self) -> &str {
        match self {
            Entity::Application(_) => "Application",
            Entity::Command { command, .. } => &command.category,
            Entity::Plugin(result) => &result.section,
        }
    }

    /// Section header this entity is grouped under.
    pub fn section(&self) -> &str {
        match self {
            Entity::Application(_) => "Applications",
            Entity::Command { .. } => "Commands",
            Entity::Plugin(result) => &result.section,
        }
    }

    /// Ordering of sections: live results first, then apps, then commands.
    pub fn section_rank(&self) -> u8 {
        match self {
            Entity::Plugin(_) => 0,
            Entity::Application(_) => 1,
            Entity::Command { .. } => 2,
        }
    }

    /// Label for the default action, shown in the footer.
    pub fn primary_action_label(&self) -> &str {
        match self {
            Entity::Application(_) => "Open",
            Entity::Command { .. } => "Run",
            Entity::Plugin(result) => result
                .actions
                .first()
                .map(|action| action.label.as_str())
                .unwrap_or("Run"),
        }
    }

    /// The effect of the default action for a live plugin result, if any.
    pub fn primary_effect(&self) -> Option<ActionEffect> {
        match self {
            Entity::Plugin(result) => result.actions.first().map(|action| action.effect.clone()),
            _ => None,
        }
    }

    /// Plugin-defined actions for the actions menu (empty otherwise).
    pub fn plugin_actions(&self) -> &[PluginAction] {
        match self {
            Entity::Plugin(result) => &result.actions,
            _ => &[],
        }
    }

    /// Glyph to render on the fallback tile, if the entity provides one.
    pub fn tile_glyph(&self) -> Option<char> {
        match self {
            Entity::Command { command, .. } => command.glyph,
            Entity::Plugin(result) => result.glyph,
            _ => None,
        }
    }

    /// Id of the owning plugin, for routing commands and view events.
    pub fn plugin_source_id(&self) -> Option<&str> {
        match self {
            Entity::Command { plugin_id, .. } => Some(plugin_id),
            Entity::Plugin(result) => Some(&result.source_id),
            _ => None,
        }
    }

    /// `(plugin_id, command_id)` when this entity is a plugin command.
    pub fn command_ref(&self) -> Option<(&str, &str)> {
        match self {
            Entity::Command {
                plugin_id, command, ..
            } => Some((plugin_id, &command.id)),
            _ => None,
        }
    }

    /// Whether this is a fallback command in its un-materialized (list) form.
    /// Such commands are hidden from the normal filtered list and only offered
    /// as fallbacks.
    pub fn is_fallback_command(&self) -> bool {
        matches!(
            self,
            Entity::Command { command, fallback_query: None, .. } if command.fallback
        )
    }

    /// The typed query baked into a fallback command, if any (used as the
    /// argument on activation instead of prompting).
    pub fn fallback_query(&self) -> Option<&str> {
        match self {
            Entity::Command {
                fallback_query: Some(query),
                ..
            } => Some(query),
            _ => None,
        }
    }

    /// If this is a fallback-capable command, materialize a fallback instance
    /// for `query` (to append at the bottom of the results).
    pub fn fallback_for(&self, query: &str) -> Option<Entity> {
        match self {
            Entity::Command {
                plugin_id,
                command,
                fallback_query: None,
            } if command.fallback => Some(Entity::Command {
                plugin_id: plugin_id.clone(),
                command: command.clone(),
                fallback_query: Some(query.to_string()),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn google_fallback() -> Entity {
        Entity::Command {
            plugin_id: "web.google".to_string(),
            command: Command {
                id: "search".to_string(),
                title: "Search Google".to_string(),
                subtitle: None,
                keywords: vec![],
                icon: None,
                glyph: None,
                category: "Web Search".to_string(),
                needs_argument: true,
                argument_placeholder: None,
                fallback: true,
            },
            fallback_query: None,
        }
    }

    #[test]
    fn fallback_materializes_on_query() {
        let command = google_fallback();
        // Hidden from the normal list until materialized.
        assert!(command.is_fallback_command());

        let fallback = command
            .fallback_for("rust iced")
            .expect("fallback should materialize");

        assert!(!fallback.is_fallback_command());
        assert_eq!(fallback.fallback_query(), Some("rust iced"));
        // The typed query is the argument, so it never prompts.
        assert!(!fallback.needs_argument());
        assert_eq!(fallback.description(), Some("rust iced"));
        assert_eq!(fallback.command_ref(), Some(("web.google", "search")));
    }
}

/// The static, searchable entries: system applications plus every command
/// registered by the plugins.
pub fn get_entities(registry: &PluginRegistry) -> Vec<Entity> {
    let mut entities: Vec<Entity> = App::lookup_applications()
        .into_iter()
        .map(Entity::Application)
        .collect();

    for (plugin_id, command) in registry.commands() {
        entities.push(Entity::Command {
            plugin_id,
            command,
            fallback_query: None,
        });
    }

    entities
}
