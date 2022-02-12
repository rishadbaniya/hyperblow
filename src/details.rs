// Struct that holds the state for Details Section of the UI
#[derive(Debug, Clone)]
pub struct Details {
    pub name: Option<String>,
    pub info_hash: Option<Vec<u8>>,
    pub total_pieces: Option<u32>,
    pub piece_length: Option<i64>,
    pub total_bytes: Option<i64>,
    pub downloaded_bytes: Option<u64>,
}

impl Default for Details {
    fn default() -> Self {
        Self {
            name: None,
            info_hash: None,
            total_bytes: None,
            downloaded_bytes: None,
            piece_length: None,
            total_pieces: None,
        }
    }
}
