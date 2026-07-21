use core::Entity;
use std::{path::Path, sync::Arc};

use anyhow::Result;
use iced::widget::{image, svg};

use crate::design_system::colors;

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
        // Use the real icon when the entity provides one, otherwise fall back
        // to a generated colored letter tile (rather than a placeholder image).
        let image_handler = match value.icon() {
            Some(image) => image.into(),
            None => IconHandle::letter(value.name()),
        };

        ListEntry {
            entity: Arc::new(value),
            image_handler,
        }
    }
}

#[derive(Clone, Debug)]
pub enum IconHandle {
    Svg(svg::Handle),
    Other(image::Handle),
    /// A generated fallback tile: an uppercase initial on a colored square.
    Letter { letter: char, color: iced::Color },
}

impl IconHandle {
    /// Build a letter tile from a label: its uppercased initial on a color
    /// deterministically derived from the label.
    pub fn letter(label: &str) -> Self {
        let letter = label
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or('?');

        IconHandle::Letter {
            letter,
            color: colors::tile_color(label),
        }
    }
}

impl From<core::Image> for IconHandle {
    fn from(value: core::Image) -> Self {
        match value {
            core::Image::Bytes(bytes) => IconHandle::Other(image::Handle::from_bytes(bytes)),
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
