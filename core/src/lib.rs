use crate::plugins::CommandEntity;

pub use crate::common::Image;
pub use crate::plugins::{ActionEffect, Plugin, PluginAction, PluginRegistry, PluginResult};
use anyhow::Result;
pub use application::App;
pub use application::Application;
pub use common::AppState;

mod application;
pub mod clipboard;
mod common;
mod plugins;
pub mod search;

const QUALIFIER: &str = "com";
const ORGANISATION: &str = "lcvitor";
const APPLICATION: &str = "iced_raycast";

#[derive(Debug, Clone)]
pub enum Entity {
    Application(App),
    Command(CommandEntity),
    /// A dynamic result produced by a [`Plugin`] for the current query.
    Plugin(PluginResult),
}

impl Entity {
    pub fn name(&self) -> &str {
        match self {
            Entity::Application(app) => app.name(),
            Entity::Command(cmd) => &cmd.name,
            Entity::Plugin(result) => &result.title,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            Entity::Application(app) => app.description(),
            Entity::Command(cmd) => cmd.description.as_deref(),
            Entity::Plugin(result) => result.subtitle.as_deref(),
        }
    }

    pub fn icon(&self) -> Option<Image> {
        match self {
            Entity::Application(app) => app.icon(),
            Entity::Command(cmd) => cmd.image.clone(),
            Entity::Plugin(result) => result.icon.clone(),
        }
    }

    pub fn execute(&self, argument: Option<String>) -> Result<()> {
        match self {
            Entity::Application(app) => app.execute(argument),
            Entity::Command(cmd) => {
                // TODO: dispatch to the owning plugin once commands become
                // plugin-provided results.
                eprintln!(
                    "Executing command {} with argument {:?}",
                    cmd.name, argument
                );
                Ok(())
            }
            // Plugin results act through their effect (see `primary_effect`),
            // not a synchronous launch.
            Entity::Plugin(_) => Ok(()),
        }
    }

    pub fn needs_argument(&self) -> bool {
        match self {
            Entity::Application(_) => false,
            Entity::Command(cmd) => cmd.needs_argument,
            Entity::Plugin(_) => false,
        }
    }

    /// Singular label shown on the right of a result row.
    pub fn kind_label(&self) -> &str {
        match self {
            Entity::Application(_) => "Application",
            Entity::Command(_) => "Command",
            Entity::Plugin(result) => &result.section,
        }
    }

    /// Plural section header this entity is grouped under.
    pub fn section(&self) -> &str {
        match self {
            Entity::Application(_) => "Applications",
            Entity::Command(_) => "Commands",
            Entity::Plugin(result) => &result.section,
        }
    }

    /// Ordering of sections in the list: plugin results first, then apps, then
    /// commands.
    pub fn section_rank(&self) -> u8 {
        match self {
            Entity::Plugin(_) => 0,
            Entity::Application(_) => 1,
            Entity::Command(_) => 2,
        }
    }

    /// Label for the default action, shown in the footer.
    pub fn primary_action_label(&self) -> &str {
        match self {
            Entity::Application(_) => "Open",
            Entity::Command(_) => "Run Command",
            Entity::Plugin(result) => result
                .actions
                .first()
                .map(|action| action.label.as_str())
                .unwrap_or("Run"),
        }
    }

    /// The effect of the default action for a plugin result, if any. Apps and
    /// commands return `None` and are launched via [`Entity::execute`] instead.
    pub fn primary_effect(&self) -> Option<ActionEffect> {
        match self {
            Entity::Plugin(result) => result.actions.first().map(|action| action.effect.clone()),
            _ => None,
        }
    }

    /// Plugin-defined actions for the actions menu (empty for apps/commands).
    pub fn plugin_actions(&self) -> &[PluginAction] {
        match self {
            Entity::Plugin(result) => &result.actions,
            _ => &[],
        }
    }

    /// Glyph to render on the fallback tile, if the entity provides one.
    pub fn tile_glyph(&self) -> Option<char> {
        match self {
            Entity::Plugin(result) => result.glyph,
            _ => None,
        }
    }
}

pub fn get_entities() -> Vec<Entity> {
    // `mut` is only needed for the debug-only fake commands below.
    #[cfg_attr(not(debug_assertions), allow(unused_mut))]
    let mut entities: Vec<Entity> = App::lookup_applications()
        .into_iter()
        .map(Entity::Application)
        .collect();

    // Placeholder commands used to exercise the UI (including the
    // needs-argument flow) while the plugin system is being built. Excluded
    // from release builds so they never ship as real entries.
    #[cfg(debug_assertions)]
    entities.extend(fake_commands());

    entities
}

#[cfg(debug_assertions)]
fn fake_commands() -> Vec<Entity> {
    vec![
        Entity::Command(CommandEntity {
            id: 0,
            name: "Fake Command One".to_string(),
            alias: None,
            description: Some("This is the first fake command.".to_string()),
            image: None,
            needs_argument: false,
        }),
        Entity::Command(CommandEntity {
            id: 1,
            name: "Fake Command Two".to_string(),
            alias: Some("fct".to_string()),
            description: Some("This is the second fake command, with an alias.".to_string()),
            image: None,
            needs_argument: false,
        }),
        Entity::Command(CommandEntity {
            id: 2,
            name: "Fake Command Three".to_string(),
            alias: None,
            description: Some("A third example of a fake command.".to_string()),
            image: None,
            needs_argument: true, // This one needs an argument
        }),
    ]
}
