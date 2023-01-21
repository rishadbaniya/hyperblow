use crate::core::state::{DownState, State};
use crate::core::tracker::Tracker;
use crate::core::File;
use crate::ArcMutex;
use futures::channel::mpsc::UnboundedReceiver;
use futures::future::join;
use futures::stream::FuturesUnordered;
use futures::{join, FutureExt, StreamExt};
use hyperblow_parser::torrent_parser::FileMeta;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot::{self, Sender};
use tokio::sync::RwLock;
use tokio::{net::UdpSocket, sync::Mutex, task::JoinHandle, time::timeout};

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
        let trackers = self.state.trackers.clone();
        let mut tracker_s: Vec<Vec<Arc<RwLock<Tracker>>>> = Vec::new(); // Stores the resolved trackers

        // According to BEP12, if announce_list field is present then the client will have to
        // ignore the announce field as the URL in the announce field is already present in the
        // announce_list
        //
        // Inside this function resolveTrackers(...) only the initial step of extracting out the
        // URLS from announce_list or announce field is considered and resolving the DNS of the URL
        // is done
        if let Some(ref d) = self.meta_info.announce_list {
            for i in d {
                let x: Vec<Arc<RwLock<Tracker>>> = {
                    let mut trackers = vec![];
                    for addrs in i {
                        // This tries to parse the given URL and if it parses successfully then
                        // signals to resolve the socket address
                        if let Ok(mut tracker) = Tracker::new(addrs) {
                            // TODO : Figure out what to do to those trackers who DNS wasn't
                            // resolved, whether to try after a certain time or what
                            if tracker.resolveSocketAddr() {
                                trackers.push(Arc::new(RwLock::new(tracker)));
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
                    tracker_s.push(vec![Arc::new(RwLock::new(tracker))]);
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
        let req = self.sendTrackersRequests(socket.clone());
        let res = self.receiveTrackersResponse(socket);
        join!(req, res);
    }

    async fn sendTrackersRequests(&self, socket: Arc<UdpSocket>) {
        // Sets the timeout duration for the requests
        let timeout_duration = |n: u64| Duration::from_secs(15 + 2 ^ n);

        //let tracker_rq = |tr: &Tracker| async {
        //    let mut _no_of_times_connect_request_timeout = 0;
        //    loop {
        //        match timeout(timeout_duration(_no_of_times_connect_request_timeout), tr.sendConnectRequest(socket.clone())) {
        //            _ => {}
        //        }
        //        //    match timeout(Duration::from_secs(15 + 2 ^ no_of_times_connect_request_timeout), receiver_borrow_mut.recv()).await {
        //    }
        //};

        //        let tracker_lock = tracker.lock().await;
        //        let socket_adr = tracker_lock.socket_adr.unwrap();
        //        drop(tracker_lock);
        //        let mut receiver_borrow_mut = receiver.borrow_mut();
        //        if let Ok(_) = connect_request(TRANS_ID, &udp_socket, &socket_adr, tracker.clone()).await {
        //            //
        //            // Waits for 15 * 2 ^ n seconds, where n is from 0 to 8 => (3840 seconds), for Connect Response to come
        //            //
        //            match timeout(Duration::from_secs(15 + 2 ^ no_of_times_connect_request_timeout), receiver_borrow_mut.recv()).await {
        //                //
        //                Ok(v) => {
        //                    no_of_times_connect_request_timeout = 0;
        //                    let buf = v.unwrap();
        //                    let mut action_bytes = &buf[0..=3];
        //                    let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes).unwrap();
        //
        //                    // Action = 0 means it's "Connect Response"
        //                    if action == 0 {
        //                        let connect_response = ConnectResponse::from(buf);
        //                        if let Ok(_) = annnounce_request(connect_response, &udp_socket, &socket_adr, details.clone(), tracker.clone()).await {}
        //                        match timeout(Duration::from_secs(10), receiver_borrow_mut.recv()).await {
        //                            Ok(v) => {
        //                                let announce_response = AnnounceResponse::new(v.as_ref().unwrap()).unwrap();
        //                                peers_sender.send(announce_response.peersAddresses).await.unwrap();
        //                                sleep(Duration::from_secs(announce_response.interval as u64)).await;
        //                            }
        //                            _ => {}
        //                        }
        //                    }
        //                }
        //                Err(_) => {
        //                    no_of_times_connect_request_timeout += 1;
        //                }
        //            };
        //        } else {
        //            // This else portion runs when a Tracker Socket cant be reached, in this condition we
        //            // will poll the tracker again after 15 * 2 ^ n
        //            //
        //            // Waits for  seconds, where n is from 0 to 8 => (3840 seconds), for Connect Response to come
        //            // println!("Some error");
        //            sleep(Duration::from_secs(14 + 2 ^ no_of_times_connect_request_timeout)).await;
        //            if !no_of_times_connect_request_timeout == 8 {
        //                no_of_times_connect_request_timeout += 1;
        //            }
        //        }
        //        sleep(Duration::from_secs(1)).await;
        //    }
        //}
    }

    // NOTE : This function must run concurrently with sendTrackersRequests function and vice versa
    async fn receiveTrackersResponse(&self, socket: Arc<UdpSocket>) {
        loop {
            // Replace this vector with bytes mutj
            let mut buf: Vec<u8> = Vec::new();
            match socket.recv_from(&mut buf).await {
                Ok((_len, ref s_addrs)) => {
                    // NOTE : I could've stored all trackers in the top scope of this
                    // receiveTrackersResponse function, so that i don't have to await. But, the
                    // problem is of BEP12, where i have to constantly arrange the Trackers, this
                    // changes the index in the self.state.trackers field
                    let trackers = self.state.trackers.lock().await;
                    for trackers in trackers.iter() {
                        for tracker in trackers {
                            if tracker.isEqualTo(s_addrs) {
                                if let Some((ref sd, _)) = tracker.lock().await.udp_channel {
                                    sd.send(buf.clone());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    //async fn () {
    //}

    //async fn sendTra

    pub async fn runDownload(&self) {
        // TODO: Run in a loop, but never return anything
        for i in 1..=20000000 {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            println!("Does the download stuff here");
        }
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
