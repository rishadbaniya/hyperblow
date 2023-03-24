#![feature(concat_idents)]

use crate::core::{peer::Peer, tracker::Tracker, File};
use crossbeam::atomic::AtomicCell;
use hyperblow::parser::torrent_parser::FileMeta;
use paste::paste;

use std::{cell::Cell, sync::Arc};
use tokio::sync::{Mutex, RwLock};

/// Used to generate getter and setter for Cell<T> types
/// Eg.
/// If there is field like
/// xyz : Cell<i32>
///
/// Then we can simply use following code in the impl block
/// cell_get_set!(xyz: i32);
///
/// It wll generate two methods to get and set value from and in the Cell
///
/// pub fn xyz(&self){/*...*/}
/// pub fn set_xyz(&self, value){/*...*/}
macro_rules! cell_get_set {
    ($field:ident: $ty:ty) => {
        pub fn $field(&self) -> $ty {
            self.$field.load()
        }

        // paste! macro is used to concatenate identifiers
        // See : https://docs.rs/paste/latest/paste
        // concate_idents! is currently unstable and only nightly availaible, so gotta paste!
        paste! {
            pub fn [<set_ $field>](&self, val: $ty) {
                self.$field.store(val)
            }
        }
    };
}

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
    pub meta_info: FileMeta,

    pub d_state: DownState,

    /// The entire file tree of the torrent files to be downloaded
    pub file_tree: Option<Arc<Mutex<File>>>,

    /// The trackers of the torrent file
    pub trackers: Arc<RwLock<Vec<Vec<Arc<Tracker>>>>>,

    /// A list of UDP ports being used by this torrent being downloaded
    /// The port at index 0, is the port used for UDP Trackers and it always exists
    pub udp_ports: Arc<Mutex<Vec<u16>>>,

    /// A list of TCP ports being used by this torrent being downloaded
    pub tcp_ports: Arc<Mutex<Vec<u16>>>,

    /// Info hash of the torrent
    pub info_hash: Vec<u8>,

    /// Stores the hash of each piece by its exact index extracted out of bencode encoded ".torrent" file
    pub pieces_hash: Vec<[u8; 20]>,

    /// All the peers of the current session
    pub peers: Arc<Mutex<Vec<Peer>>>,

    /// Total session time that torrent has been active in seconds
    pub uptime: AtomicCell<usize>,

    /// Total bytes downloaded
    pub bytes_complete: AtomicCell<usize>,

    // Total downloaded pieces
    pub pieces_downloaded: AtomicCell<usize>,
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

    cell_get_set!(uptime: usize);

    cell_get_set!(bytes_complete: usize);

    cell_get_set!(pieces_downloaded: usize);
}
