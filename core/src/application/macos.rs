use crate::Application;
use app_info::{AppInfo, get_installed_apps};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct MacOSApplication {
    // Wrap the crate's info struct
    inner: AppInfo,
}

impl Application for MacOSApplication {
    fn lookup_applications() -> Vec<Self>
    where
        Self: Sized,
    {
        // 64 is a standard icon size for launchers
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
        // app_info doesn't explicitly provide aliases,
        // but you could return the name again or None
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

            // Encode the raw RGBA pixels from app_info into a PNG format
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

    fn execute(&self, arg: Option<&str>) -> anyhow::Result<()> {
        // On macOS, the best way to launch a .app bundle is via the 'open' command
        // This handles focus and instance management correctly.
        let mut cmd = Command::new("open");

        cmd.arg("-a").arg(&self.inner.path);

        if let Some(a) = arg {
            cmd.arg("--args").arg(a);
        }

        cmd.spawn()?;
        Ok(())
    }
}
