use crate::common::Image;

#[derive(Debug, Clone)]
pub struct CommandEntity {
    pub id: u64,
    // plugin_ref: Plugin_Ref,
    pub name: String,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub image: Option<Image>,
    pub needs_argument: bool,
}
