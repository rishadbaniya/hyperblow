use std::collections::HashSet;

/// Struct that holds the state and details of the torrent that is being
/// downloaded
#[derive(Debug, Clone)]
pub struct Details {
    /// Name of the torrent
    pub name: Option<String>,
    /// Info hash of the torrent
    pub info_hash: Option<Vec<u8>>,
    /// Total no of pieces
    pub total_pieces: u32,
    pub piece_length: Option<i64>,
    pub total_bytes: Option<i64>,
    pub downloaded_bytes: Option<u64>,
    /// Hash of all the pieces in same index as piece index
    pub pieces_hash: Vec<[u8; 20]>,
    /// Zero based index of the pieces downloaded
    pub pieces_downloaded: HashSet<u32>,
    /// Zero based index of the piece downloading
    pub pieces_requested: HashSet<u32>,
}

impl Default for Details {
    fn default() -> Self {
        let pieces_hash = Vec::new();
        Self {
            name: None,
            info_hash: None,
            total_bytes: None,
            downloaded_bytes: None,
            piece_length: None,
            total_pieces: 0,
            pieces_hash,
            pieces_downloaded: HashSet::new(),
            pieces_requested: HashSet::new(),
        }
    }
}
