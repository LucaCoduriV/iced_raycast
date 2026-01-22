#[derive(Debug, Clone)]
pub enum Image {
    Data(Vec<u8>),
    Path(String),
}
