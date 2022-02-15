use std::collections::HashSet;

// Struct that holds the state for Details Section of the UI
#[derive(Debug, Clone)]
pub struct Details {
    pub name: Option<String>,
    pub info_hash: Option<Vec<u8>>,
    pub total_pieces: Option<u32>,
    pub piece_length: Option<i64>,
    pub total_bytes: Option<i64>,
    pub downloaded_bytes: Option<u64>,
    pub pieces_hash: Vec<[u8; 20]>,
    /// Zero based index of the pieces downloaded
    pub pieces_downloaded: HashSet<u32>,
    /// Zero based index of the piece downloading
    pub pieces_requested: HashSet<u32>,
}

impl Default for Details {
    fn default() -> Self {
        let pieces_hash = Vec::new();
        let pieces_downloaded = HashSet::new();
        let pieces_requested = HashSet::new();
        Self {
            name: None,
            info_hash: None,
            total_bytes: None,
            downloaded_bytes: None,
            piece_length: None,
            total_pieces: None,
            pieces_hash,
            pieces_downloaded,
            pieces_requested,
        }
    }
}
