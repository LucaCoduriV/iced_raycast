use crate::{Application, Image};

#[derive(Clone, Debug)]
pub struct WindowsApplication {}

impl Application for WindowsApplication {
    fn lookup_applications() -> Vec<Self>
    where
        Self: Sized,
    {
        Vec::new()
    }

    fn name(&self) -> &str {
        "Salut"
    }

    fn alias(&self) -> Option<&str> {
        None
    }

    fn description(&self) -> Option<&str> {
        None
    }

    fn icon(&self) -> Option<Image> {
        None
    }

    fn execute(&self, arg: Option<String>) -> anyhow::Result<()> {
        Ok(())
    }
}
