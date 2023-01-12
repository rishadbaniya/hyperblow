use crate::core::state::{DownState, State};
use crate::core::tracker::Tracker;
use crate::core::File;
use crate::ArcMutex;
use futures::future::join_all;
use futures::join;
use hyperblow_parser::torrent_parser::FileMeta;
use std::sync::Arc;
use tokio::{net::UdpSocket, sync::Mutex, task::JoinHandle};

#[derive(Debug)]
pub enum TError {
    NoTrackerResolved,
}

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
    pub async fn new(path: &String) -> Option<TorrentFile> {
        match FileMeta::fromTorrentFile(&path) {
            Ok(meta_info) => {
                let info_hash = meta_info.generateInfoHash();
                let pieces_hash = meta_info.getPiecesHash();

                let state = Arc::new(State {
                    d_state: DownState::Unknown,
                    file_tree: Some(TorrentFile::generateFileTree(&meta_info).await),
                    trackers: ArcMutex!(vec![]),
                    udp_ports: ArcMutex!(Vec::new()),
                    tcp_ports: ArcMutex!(Vec::new()),
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

    // NOTE : This function is assumed to be called once in the download session
    /// Creates objects of [Tracker] by extracting out all the Trackers from "announce" and "announce-list" field
    /// and then resolves their address through DNS lookup
    async fn resolveTrackers(&self) -> Result<(), TError> {
        let trackers = self.state.trackers.clone(); // Pointer to store the trackers
        let mut tracker_s: Vec<Vec<Tracker>> = Vec::new(); // Stores the resolved trackers

        // According to BEP12, if announce_list field is present then the client will have to

        // ignore the announce field as the URL in the announce field is already present in the
        // announce_list
        //
        //
        // Step -1 :
        //
        // Create a Tracker instance, parse the given string and then resolve the socket
        // address for both cases, when there is only announce field and when there is
        // announce_list field as well
        if let Some(ref d) = self.meta_info.announce_list {
            for i in d {
                let x: Vec<Tracker> = {
                    let mut trackers = vec![];
                    for addrs in i {
                        // This tries to parse the given URL and if it parses successfully then
                        // signals to resolve the socket address
                        if let Ok(mut tracker) = Tracker::new(addrs) {
                            // TODO : Figure out what to do to those trackers who DNS wasn't
                            // resolved, whether to try after a certain time or what
                            if tracker.resolveSocketAddr() {
                                trackers.push(tracker);
                            }
                        }
                    }
                    trackers
                };
                tracker_s.push(x);
            }
        } else {
            if let Ok(mut tracker) = Tracker::new(&self.meta_info.announce) {
                if tracker.resolveSocketAddr() {
                    tracker_s.push(vec![tracker]);
                }
            }
        }

        return if tracker_s.len() == 0 {
            Err(TError::NoTrackerResolved)
        } else {
            *(trackers.lock().await) = tracker_s;
            return Ok(());
        };
    }

    pub async fn generateFileTree(meta: &FileMeta) -> Arc<Mutex<File>> {
        // We'll consider the root file to be named "."
        File::new(meta, &".".to_owned()).await.unwrap()
    }

    pub async fn getUDPSocket(&mut self) -> Arc<UdpSocket> {
        // TODO : Currently this function exhaustively checks for each port and tries to
        // give one of the ports incrementing from 6881
        let mut port = 6881;
        // TODO: Get a list of all the ports used by the entire application as well,
        // i.e store a global use  of entire sockets somewhere in a global state
        //
        // Gets a port that is not used by the application
        loop {
            let adr = format!("0.0.0.0:{port}");
            match UdpSocket::bind(adr).await {
                Ok(socket) => {
                    let mut udp_ports = self.state.udp_ports.lock().await;
                    udp_ports.push(port);
                    return Arc::new(socket);
                }
                Err(e) => {
                    println!("{:?}", e.to_string());
                    port = port + 1;
                }
            }
        }
    }

    /// Starts all the trackers parallel with each other
    // TODO : Handle the case for support of TCP Trackers as well
    pub async fn runTrackers(&self, sockets: Arc<UdpSocket>) {
        let trackers = self.state.trackers.clone();
        println!("RAN USING TRACKERS AT RANDOM ORDER");
    }

    pub async fn runDownload(&self) {
        println!("RAN DOWNLOADE AT RANDOM ORDER");
    }

    // TODO : Add examples for the rust docs
    // /// Starts to download the torrent, it will keep on mutating the "state" field as it
    // /// makes progress, and if the torrent needs to be pause or started, one can use the method on
    // /// that State instance
    // ///
    // /// NOTE : While using this method, one must clone and keep a Arc pointer of "state" field,
    // /// so that they can use it later on to display the UI or the data changed
    pub fn run(mut torrent: TorrentFile) -> JoinHandle<()> {
        let rt = async move {
            println!("HEY THERE");

            // A UDP socket for Trackers, not just a single tracker
            let t_udp_socket = torrent.getUDPSocket().await;

            //let tasks = vec![];
            // Collects all the tasks
            // 1. Running the trackers
            // 2. Running the download process
            //
            // At last run them in parallel, through some ways such as join_all(Uses abstraction upon FuturesUnordered) or FuturesUnordered directly

            if let Ok(_) = torrent.resolveTrackers().await {
                let run_trackers = torrent.runTrackers(t_udp_socket.clone());
                let run_download = torrent.runDownload();
            } else {
                // Handle what happens when none of the trackers DNS are resolved
            }
        };
        tokio::spawn(rt)
    }
}
