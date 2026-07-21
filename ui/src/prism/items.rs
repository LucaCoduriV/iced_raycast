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

    /// Singular label shown on the right of the row ("Application", "Calculator").
    pub fn kind_label(&self) -> &str {
        self.entity.kind_label()
    }

    /// Plural section header this entry groups under.
    pub fn section(&self) -> &str {
        self.entity.section()
    }

    /// Section ordering key (plugins first, then apps, then commands).
    pub fn section_rank(&self) -> u8 {
        self.entity.section_rank()
    }

    /// Label for the default action, shown in the footer.
    pub fn primary_action_label(&self) -> &str {
        self.entity.primary_action_label()
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
        // Pick the tile: an entity-provided glyph (e.g. '=' for the
        // calculator), then a real icon, then a generated letter tile.
        let image_handler = if let Some(glyph) = value.tile_glyph() {
            IconHandle::Letter {
                letter: glyph,
                color: colors::tile_color(value.section()),
            }
        } else {
            match value.icon() {
                Some(image) => image.into(),
                None => IconHandle::letter(value.name()),
            }
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
