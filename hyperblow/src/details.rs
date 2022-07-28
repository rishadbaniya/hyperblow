use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Struct that holds the state and details of the torrent that is being
/// downloaded
#[derive(Debug, Clone)]
pub struct Details {
    /// Name of the torrent
    pub name: Option<String>,
    /// Info hash of the torrent
    pub info_hash: Option<[u8; 20]>,
    /// Total no of pieces
    pub total_pieces: u32,
    /// Length of each piece
    pub piece_length: Option<i64>,
    /// Total size in bytes
    pub total_bytes: i64,
    /// Total bytes that has been downloaded
    pub downloaded_bytes: i64,
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
            total_bytes: 0,
            downloaded_bytes: 0,
            piece_length: None,
            total_pieces: 0,
            pieces_hash,
            pieces_downloaded: HashSet::new(),
            pieces_requested: HashSet::new(),
        }
    }
}
