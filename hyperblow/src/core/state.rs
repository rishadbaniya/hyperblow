use crate::core::{tracker::Tracker, File};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug)]
pub enum DownState {
    /// It means the torrent is currently downloading
    Downloading,
    /// It means the download of the torrent is currenlty stopped
    Stopped,
    /// It means the state is unknown, it might be requesting data from some tracker or doing
    /// something else, but not downloading the data of the torrent and not in a paused state
    Unknown,
}

/// A thread shareable state of the torrent being downloaded.
///
/// Data that can be showed to the user is stored in [State]
#[derive(Debug)]
pub struct State {
    pub d_state: DownState,

    /// The entire file tree of the torrent files to be downloaded
    pub file_tree: Option<Arc<Mutex<File>>>,

    /// The trackers of the torrent file
    pub trackers: Arc<RwLock<Vec<Vec<Arc<Tracker>>>>>,

    /// A list of UDP ports being used by this torrent being downloaded
    ///
    /// The port at index 0, is the port used for UDP Trackers and it always exists
    pub udp_ports: Arc<Mutex<Vec<u16>>>,

    /// A list of TCP ports being used by this torrent being downloaded
    pub tcp_ports: Arc<Mutex<Vec<u16>>>,

    /// Info hash of the torrent
    pub info_hash: Vec<u8>,

    /// Total size of the entire torrent file in bytes
    pub total_size: usize,

    /// Stores the hash of each piece by its exact index extracted out of bencode encoded ".torrent" file
    pub pieces_hash: Vec<[u8; 20]>,
}

impl State {
    /// Stop the download of the torrent
    fn stop() {
        // Code to pause the download
    }

    /// Start the download of the torrent
    fn start() {
        // Code to resume the download
    }
}
