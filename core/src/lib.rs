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

#[derive(Debug, Clone)]
pub enum Entity {
    Application(App),
    Command(CommandEntity),
}

impl Entity {
    pub fn name(&self) -> &str {
        match self {
            Entity::Application(app) => app.name(),
            Entity::Command(_cmd) => todo!(), // Assuming CommandEntity has a name field
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            Entity::Application(app) => app.description(),
            Entity::Command(_cmd) => todo!(),
        }
    }

    pub fn icon(&self) -> Option<Image> {
        match self {
            Entity::Application(app) => app.icon(),
            Entity::Command(_cmd) => todo!(),
        }
    }

    pub fn execute(&self) -> Result<()> {
        match self {
            Entity::Application(app) => app.execute(None),
            Entity::Command(_) => todo!(),
        }
    }
}

pub fn get_entities() -> Vec<Entity> {
    App::lookup_applications()
        .into_iter()
        .map(Entity::Application)
        .collect()
}
