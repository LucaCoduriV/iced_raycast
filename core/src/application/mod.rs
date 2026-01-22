#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::LinuxApplication as ConcreteApp;

use anyhow::Result;

use crate::common::Image;

pub type App = ConcreteApp;

#[allow(dead_code)]
pub trait Application {
    fn lookup_applications() -> Vec<Self>
    where
        Self: Sized;

    fn name(&self) -> &str;
    fn alias(&self) -> Option<&str>;
    fn description(&self) -> Option<&str>;
    fn icon(&self) -> Option<Image>;
    fn execute(&self, arg: Option<&str>) -> Result<()>;
}

pub fn get_apps() -> Vec<App> {
    ConcreteApp::lookup_applications()
}
