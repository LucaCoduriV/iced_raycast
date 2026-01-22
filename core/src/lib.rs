use crate::plugins::CommandEntity;

pub use application::App;
pub use application::Application;

mod application;
mod common;
mod plugins;

#[derive(Debug, Clone)]
pub enum Entity {
    Application(App),
    Command(CommandEntity),
}

pub fn get_entities() -> Vec<Entity> {
    App::lookup_applications()
        .into_iter()
        .map(Entity::Application)
        .collect()
}
