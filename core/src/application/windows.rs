use crate::{Application, Image};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct WindowsApplication {
    pub name: String,
    pub path: PathBuf,
    pub icon: Option<Image>,
}

impl Application for WindowsApplication {
    fn lookup_applications() -> Vec<Self>
    where
        Self: Sized,
    {
        app_info::get_installed_apps(32)
            .map(|installed_apps| {
                installed_apps
                    .into_iter()
                    .map(|app| {
                        let icon = app.icon.map(|i| Image::Rgba(i.width, i.height, i.pixels));

                        WindowsApplication {
                            name: app.name,
                            path: app.path,
                            icon,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn alias(&self) -> Option<&str> {
        None
    }

    fn description(&self) -> Option<&str> {
        self.path.to_str()
    }

    fn icon(&self) -> Option<Image> {
        self.icon.clone()
    }

    fn execute(&self, _arg: Option<String>) -> anyhow::Result<()> {
        std::process::Command::new("cmd")
            .args(["/c", "start", ""])
            .arg(&self.path)
            .spawn()?;

        Ok(())
    }
}
