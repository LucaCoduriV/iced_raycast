use crate::plugins::CommandEntity;

pub use crate::common::Image;
use anyhow::Result;
pub use application::App;
pub use application::Application;
pub use common::AppState;

mod application;
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
}

impl Entity {
    pub fn name(&self) -> &str {
        match self {
            Entity::Application(app) => app.name(),
            Entity::Command(cmd) => &cmd.name,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            Entity::Application(app) => app.description(),
            Entity::Command(cmd) => cmd.description.as_deref(),
        }
    }

    pub fn icon(&self) -> Option<Image> {
        match self {
            Entity::Application(app) => app.icon(),
            Entity::Command(cmd) => cmd.image.clone(),
        }
    }

    pub fn execute(&self, argument: Option<String>) -> Result<()> {
        match self {
            Entity::Application(app) => app.execute(argument),
            Entity::Command(cmd) => {
                // TODO: dispatch to the owning plugin once the plugin system lands.
                eprintln!(
                    "Executing command {} with argument {:?}",
                    cmd.name, argument
                );
                Ok(())
            }
        }
    }

    pub fn needs_argument(&self) -> bool {
        match self {
            Entity::Application(_) => false,
            Entity::Command(cmd) => cmd.needs_argument,
        }
    }
}

pub fn get_entities() -> Vec<Entity> {
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
