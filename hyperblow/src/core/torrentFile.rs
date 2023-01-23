#![allow(unused_must_use)]

use crate::core::state::{DownState, State};
use crate::core::tracker::Tracker;
use crate::core::File;
use crate::{ArcMutex, ArcRwLock};
use futures::future::join_all;
use hyperblow_parser::torrent_parser::FileMeta;
use std::sync::Arc;
use tokio::join;
use tokio::sync::RwLock;
use tokio::{net::UdpSocket, sync::Mutex, task::JoinHandle};

const TRANS_ID: i32 = 10;

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

    /// DataStructure that holds metadata about the date encoded inside of ".torrent" file
    pub meta_info: FileMeta,

    /// Stores the total no of pieces
    pub pieces_count: usize,

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
                let pieces_count = pieces_hash.len();
                let d_state = DownState::Unknown;
                let file_tree = Some(TorrentFile::generateFileTree(&meta_info).await);
                let trackers = ArcRwLock!(Vec::new());
                let udp_ports = ArcMutex!(Vec::new());
                let tcp_ports = ArcMutex!(Vec::new());
                let total_size = 0; // TODO : Replace it with actual total size of the torrent

                let state = Arc::new(State {
                    d_state,
                    file_tree,
                    trackers,
                    udp_ports,
                    tcp_ports,
                    info_hash,
                    total_size,
                    pieces_hash,
                });

                Some(TorrentFile {
                    path: path.to_string(),
                    pieces_count,
                    meta_info,
                    state,
                })
            }
            _ => None,
        }
    }

    // NOTE : This function is assumed to be called once in the download session
    /// Creates objects of [Tracker] by extracting out all the Trackers from "announce" and "announce-list" field
    /// and then resolves their address through DNS lookup
    async fn resolveTrackers(&self) -> Result<(), TError> {
        let trackers = self.state.trackers.clone();
        let mut tracker_s: Vec<Vec<Arc<Tracker>>> = Vec::new(); // Stores the resolved trackers

        // According to BEP12, if announce_list field is present then the client will have to
        // ignore the announce field as the URL in the announce field is already present in the
        // announce_list
        //
        // Inside this function resolveTrackers(...) only the initial step of extracting out the
        // URLS from announce_list or announce field is considered and resolving the DNS of the URL
        // is done
        if let Some(ref d) = self.meta_info.announce_list {
            for i in d {
                let x: Vec<Arc<Tracker>> = {
                    let mut trackers = vec![];
                    for addrs in i {
                        // This tries to parse the given URL and if it parses successfully then
                        // signals to resolve the socket address
                        let torrent_state = self.state.clone();
                        if let Ok(mut tracker) = Tracker::new(addrs, torrent_state) {
                            // TODO : Figure out what to do to those trackers who DNS wasn't
                            // resolved, whether to try after a certain time or what
                            if tracker.resolveSocketAddr() {
                                trackers.push(Arc::new(tracker));
                            }
                        }
                    }
                    trackers
                };
                tracker_s.push(x);
            }
        } else {
            let ref addrs = self.meta_info.announce;
            let torrent_state = self.state.clone();
            if let Ok(mut tracker) = Tracker::new(addrs, torrent_state) {
                if tracker.resolveSocketAddr() {
                    tracker_s.push(vec![Arc::new(tracker)]);
                }
            }
        }

        return if tracker_s.len() == 0 {
            Err(TError::NoTrackerResolved)
        } else {
            *(trackers.write().await) = tracker_s;
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

    /// It runs all the trackers concurrently.
    ///
    /// This function must run concurrently with receive_trackers_response() function
    async fn send_trackers_requests(&self, socket: Arc<UdpSocket>) {
        // Run i.e invoke the run() method of all the Trackers and then push the future
        let mut all_trackers_task = Vec::new();

        let trackers = self.state.trackers.read().await;
        for trackers in trackers.iter() {
            for tracker in trackers {
                let _socket = socket.clone();
                let _tracker = tracker.clone();
                all_trackers_task.push(async move { _tracker.run(_socket).await });
            }
        }

        // Polls all future to run them concurrently
        join_all(all_trackers_task).await;
    }

    /// It simply waits on the UDP, socket. When some data arrives on the socket, it simply
    /// reads that data, length, SocketAddr from which the data came and thereafter it sends the
    /// data to the required channel of tracker that's waiting for data
    ///
    /// This function must run concurrently with send_trackers_requests() function
    async fn receive_trackers_response(&self, socket: Arc<UdpSocket>) {
        loop {
            // A buffer of 4KiB capacity
            let mut buf = [0; 4096];
            match socket.recv_from(&mut buf).await {
                Ok((len, ref s_addrs)) => {
                    // NOTE : I could've stored all trackers in the top scope of this
                    // receive_trackers_response() function, so that i don't have to await. But, the
                    // problem is of BEP12, where i have to constantly arrange the Trackers, this
                    // changes the index in the self.state.trackers field
                    let trackers = self.state.trackers.read().await;
                    for trackers in trackers.iter() {
                        for tracker in trackers {
                            if tracker.isEqualTo(s_addrs) {
                                if let Some((ref sd, _)) = tracker.udp_channel {
                                    if !sd.is_closed() {
                                        let mut buf = buf.to_vec();
                                        buf.truncate(len);
                                        sd.send(buf); // TODO : send() return Result<T>, might need to make use of Err ?. Figure out
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // TODO : Handle the case for support of TCP Trackers as well
    // TorrentFile
    //Currently, im assuming that, the index of trackers is constant in state field of
    // Algorithm to run trackers;
    // Count total trackers
    pub async fn runTrackers(&self, socket: Arc<UdpSocket>) {
        // Running of trackers is divided into two sub tasks
        // 1. Sending trackers requests
        // 2. Receiving trackers response
        //
        // One is, sending Trackers requests and other is
        // If there are 'n' no of trackers, then
        // In the first async task of 'req', it spawns 'n' no of tasks within itself, and these each task make a
        // request and for every response that comes in the socket, its handled by second async
        // task of 'res
        let req = self.send_trackers_requests(socket.clone());
        let res = self.receive_trackers_response(socket);
        join!(req, res);
    }

    pub async fn runDownload(&self) {
        // TODO: Run in a loop, but never return anything
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
            //let concurrent = FuturesUnordered::new();

            if let Ok(_) = torrent.resolveTrackers().await {
                let run_trackers = torrent.runTrackers(t_udp_socket.clone());
                let run_download = torrent.runDownload();

                // Run both
                // 1. Requesting and resolving the trackers
                // 2. Downloading from the peers
                join!(run_trackers, run_download);
            } else {
                // Handle what happens when none of the trackers DNS are resolved
            }
        };
        tokio::spawn(rt)
    }
}
