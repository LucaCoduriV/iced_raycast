use core::Entity;
use std::sync::Arc;

use anyhow::Result;
use iced::widget::image;

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

    pub fn icon(&self) -> image::Handle {
        match self
            .entity
            .icon()
            .unwrap_or(core::Image::Path("assets/icon_placeholder.png".to_string()))
        {
            core::Image::Data(bytes) => image::Handle::from_bytes(bytes.clone()),
            core::Image::Path(path) => image::Handle::from_path(path),
        }
    }

    pub fn execute(&self) -> Result<()> {
        self.entity.execute()
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
