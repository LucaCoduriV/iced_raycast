use crate::Application;
use app_info::{AppInfo, get_installed_apps};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct MacOSApplication {
    inner: AppInfo,
}

impl Application for MacOSApplication {
    fn lookup_applications() -> Vec<Self>
    where
        Self: Sized,
    {
        get_installed_apps(64)
            .unwrap_or_default()
            .into_iter()
            .map(|info| MacOSApplication { inner: info })
            .collect()
    }

    fn name(&self) -> &str {
        &self.inner.name
    }

    fn alias(&self) -> Option<&str> {
        None
    }

    fn description(&self) -> Option<&str> {
        None
    }

    fn icon(&self) -> Option<crate::Image> {
        self.inner.icon.as_ref().map(|icon_data| {
            // Option A: If your UI handles raw RGBA pixels
            // Image::Data(icon_data.data.clone())

            // Option B: Convert raw pixels to encoded PNG bytes (Most common for UI kits)
            let mut png_bytes: Vec<u8> = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);

            let _ = image::ImageEncoder::write_image(
                encoder,
                &icon_data.pixels,
                icon_data.width,
                icon_data.height,
                image::ExtendedColorType::Rgba8,
            );

            crate::Image::Data(png_bytes)
        })
    }

    fn execute(&self, arg: Option<String>) -> anyhow::Result<()> {
        let mut cmd = Command::new("open");

        cmd.arg("-a").arg(&self.inner.path);

        if let Some(a) = arg {
            if !a.is_empty() {
                cmd.arg("--args").arg(a);
            }
        }

        cmd.spawn()?;
        Ok(())
    }
}
