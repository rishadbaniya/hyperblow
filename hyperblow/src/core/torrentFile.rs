use crate::core::state::{DownState, State};
use crate::core::tracker::Tracker;
use crate::core::File;
use crate::ArcMutex;
use hyperblow_parser::torrent_parser::FileMeta;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

// TODO : Figure out a way to find which piece index and its data falls under a certain file or
// folder we want to download and how should one approach to download that shit
// TODO : Create a file tree generated from reading the torrent file
// TODO : Add Global State, by global state i mean add those values that change during the runtime
// and is crucial to show to the user as well, such as downloaded pieces, their index, trackers and
// their iformations and all other data related to it

// TODO : Find the folder to save the data
// TODO : Create the DataStructure in such a way that it could resume the download later on as well

#[derive(Debug)]
pub struct TorrentFile {
    /// Path of the torrent file
    pub path: String,

    /// Info hash of the torrent
    pub info_hash: Vec<u8>,

    /// DataStructure that holds metadata about the date encoded inside of ".torrent" file
    pub meta_info: FileMeta,

    /// Stores the hash of each piece by its exact index extracted out of bencode encoded ".torrent" file
    pub pieces_hash: Vec<[u8; 20]>,

    /// Stores the total no of pieces
    pub piecesCount: usize,

    /// Total size of the entire torrent file in bytes
    pub totalSize: usize,

    /// The data that changes during runtime and gets observed by other entity to display the
    /// progress.
    ///
    /// For eg. A UI library is keeping track of the changes in "state" every 2 seconds and displaying the UI,
    /// here we are making changes in the state field continuosly
    pub state: Arc<State>,
}

/// TODO: Implement DHT(Distributed Hash Table) as well
impl TorrentFile {
    // TODO : Return error on error generated rather than this Option<T>
    /// It will try to parse the given the path of the torrent file and create a new data structure
    /// from the Torrent file
    pub fn new(path: &String) -> Option<TorrentFile> {
        match FileMeta::fromTorrentFile(&path) {
            Ok(meta_info) => {
                let info_hash = meta_info.generateInfoHash();
                let pieces_hash = meta_info.getPiecesHash();
                let state = Arc::new(State {
                    d_state: DownState::Unknown,
                    file_tree: Some(TorrentFile::generateFileTree(&meta_info)),
                    trackers: None,
                });

                Some(TorrentFile {
                    path: path.to_string(),
                    info_hash,
                    piecesCount: pieces_hash.len(),
                    pieces_hash,
                    meta_info,
                    state,
                    totalSize: 0, // TODO : Replace it with actual total size of the torrent
                })
            }
            _ => None,
        }
    }

    /// Creates objects of [Tracker] by extracting out all the Trackers from "announce" and "announce-list" field
    /// and then resolves their address through DNS lookup
    //pub fn resolveTrackers(&self) {
    // According to BEP12, if announce_list field is present then the client will have to

    //     // ignore the announce field as the URL in the announce field is already present in the
    //     // announce_list
    //     //
    //     //
    //     // Step -1 :
    //     //
    //     // Create a Tracker instance, parse the given string and then resolve the socket
    //     // address for both cases, when there is only announce field and when there is
    //     // announce_list field as well
    //     if let Some(ref d) = self.meta_info.announce_list {
    //         for i in d {
    //             let x: Vec<Tracker> = {
    //                 let mut trackers = vec![];
    //                 for addrs in i {
    //                     // This tries to parse the given URL and if it parses successfully then
    //                     // signals to resolve the socket address
    //                     if let Ok(mut tracker) = Tracker::new(addrs) {
    //                         tracker.resolveSocketAddr();
    //                         trackers.push(tracker);
    //                     }
    //                 }
    //                 trackers
    //             };

    //             self.trackers.blocking_lock().push(x);
    //         }
    //     } else {
    //         if let Ok(mut tracker) = Tracker::new(&self.meta_info.announce) {
    //             tracker.resolveSocketAddr();
    //             self.trackers.blocking_lock().push(vec![tracker]);
    //         }
    //     }
    // }

    pub fn generateFileTree(meta: &FileMeta) -> Arc<Mutex<File>> {
        // We'll consider the root file to be named "."
        File::new(meta, &".".to_owned()).unwrap()
    }

    /// Starts to download the torrent, it will keep on mutating the "state" field as it
    /// makes progress, and if the torrent needs to be pause or started, one can use the method on
    /// that State instance
    ///
    /// NOTE : While using this method, one must clone and keep a Arc pointer of "state" field,
    /// so that they can use it later on to display the UI or the data changed
    fn download(torrentFile: TorrentFile) -> JoinHandle<()> {
        let rt = async move {
            // All the task goes here
        };
        tokio::spawn(rt)
    }
}
