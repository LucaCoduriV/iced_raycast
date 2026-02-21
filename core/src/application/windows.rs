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
        let mut apps = Vec::new();

        // On demande à app_info de récupérer les apps avec des icônes de 32x32 pixels
        if let Ok(installed_apps) = app_info::get_installed_apps(32) {
            for app in installed_apps {
                // On convertit l'icône retournée par app_info en notre type Image::Rgba
                let icon = app.icon.map(|i| Image::Rgba(i.width, i.height, i.pixels));

                apps.push(WindowsApplication {
                    name: app.name,
                    path: app.path,
                    icon,
                });
            }
        }

        apps
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
        // Lancer l'application via cmd /c start
        std::process::Command::new("cmd")
            .args(["/c", "start", ""])
            .arg(&self.path)
            .spawn()?;

        Ok(())
    }
}
