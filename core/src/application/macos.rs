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
        // Hand the UI raw RGBA pixels directly (same as the Windows backend),
        // avoiding a PNG encode/decode round-trip per icon.
        self.inner.icon.as_ref().map(|icon_data| {
            crate::Image::Rgba(icon_data.width, icon_data.height, icon_data.pixels.clone())
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
