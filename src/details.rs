// Struct that holds the state for Details Section of the UI
#[derive(Debug, Clone)]
pub struct Details {
    pub name: Option<String>,
    pub info_hash: Option<Vec<u8>>,
}

impl Default for Details {
    fn default() -> Self {
        Self { name: None, info_hash: None }
    }
}
