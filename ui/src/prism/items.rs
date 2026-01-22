use core::Entity;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ListEntry {
    pub entity: Arc<Entity>,
}

impl ListEntry {
    pub fn name(&self) -> &str {
        self.entity.as_ref().name()
    }

    pub fn description(&self) -> Option<&str> {
        self.entity.as_ref().description()
    }

    pub fn kind(&self) -> &str {
        match self.entity.as_ref() {
            Entity::Application(_) => "Application",
            Entity::Command(_) => "Command",
        }
    }

    pub fn execute(&self) {
        self.entity.execute().unwrap()
    }
}

impl From<Entity> for ListEntry {
    fn from(value: Entity) -> Self {
        match &value {
            Entity::Application(_) => ListEntry {
                entity: Arc::new(value),
            },
            Entity::Command(_) => ListEntry {
                entity: Arc::new(value),
            },
        }
    }
}
