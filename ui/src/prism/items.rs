use core::Entity;
use std::{path::Path, sync::Arc};

use anyhow::Result;
use iced::widget::{image, svg};

#[derive(Clone, Debug)]
pub struct ListEntry {
    pub entity: Arc<Entity>,
    image_handler: IconHandle,
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

    pub fn icon(&self) -> IconHandle {
        self.image_handler.clone()
    }

    pub fn execute(&self, arg: Option<String>) -> Result<()> {
        self.entity.execute(arg)
    }
}

impl From<Entity> for ListEntry {
    fn from(value: Entity) -> Self {
        let image_handler = match value
            .icon()
            .unwrap_or(core::Image::Path("assets/icon_placeholder.png".to_string()))
        {
            core::Image::Data(bytes) => IconHandle::Other(image::Handle::from_bytes(bytes.clone())),
            core::Image::Rgba(width, height, pixels) => {
                IconHandle::Other(image::Handle::from_rgba(width, height, pixels))
            }
            core::Image::Path(path) => {
                let path_obj = Path::new(&path);
                match path_obj.extension().and_then(|s| s.to_str()) {
                    Some("svg") => IconHandle::Svg(svg::Handle::from_path(path_obj)),
                    _ => IconHandle::Other(image::Handle::from_path(path_obj)),
                }
            }
        };

        match &value {
            Entity::Application(_) => ListEntry {
                entity: Arc::new(value),
                image_handler,
            },
            Entity::Command(_) => ListEntry {
                entity: Arc::new(value),
                image_handler,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub enum IconHandle {
    Svg(svg::Handle),
    Other(image::Handle),
}

impl From<core::Image> for IconHandle {
    fn from(value: core::Image) -> Self {
        match value {
            core::Image::Data(bytes) => IconHandle::Other(image::Handle::from_bytes(bytes.clone())),
            core::Image::Rgba(width, height, pixels) => {
                IconHandle::Other(image::Handle::from_rgba(width, height, pixels))
            }
            core::Image::Path(path) => {
                let path_obj = Path::new(&path);
                match path_obj.extension().and_then(|s| s.to_str()) {
                    Some("svg") => IconHandle::Svg(svg::Handle::from_path(path_obj)),
                    _ => IconHandle::Other(image::Handle::from_path(path_obj)),
                }
            }
        }
    }
}
