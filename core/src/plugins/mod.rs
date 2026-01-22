use crate::common::Image;

#[derive(Debug, Clone)]
pub struct CommandEntity {
    id: u64,
    // plugin_ref: Plugin_Ref,
    name: String,
    alias: Option<String>,
    description: Option<String>,
    image: Option<Image>,
}
