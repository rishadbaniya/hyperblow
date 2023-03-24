// TODO : Handle the case for support of TCP Trackers as well
// TODO : Figure out a way to find which piece index and its data falls under a certain file or
// folder we want to download and how should one approach to download that shit
// TODO : Create a file tree generated from reading the torrent file
// TODO : Add Global State, by global state i mean add those values that change during the runtime
// and is crucial to show to the user as well, such as downloaded pieces, their index, trackers and
// their iformations and all other data related to it

// TODO : Find the folder to save the data
// TODO : Create the DataStructure in such a way that it could resume the download later on as well
// TODO : Return error on error generated rather than this Option<T> on TorrentFile::new()
#![allow(unused_must_use)]
use super::peer::Peer;
use crate::{
    core::{
        state::{DownState, State},
        tracker::Tracker,
        File,
    },
    ACell, ArcMutex, ArcRwLock,
};
use crossbeam::atomic::AtomicCell;
use futures::future::{join, join_all};
use hyperblow::parser::torrent_parser::FileMeta;
use std::{cell::Cell, sync::Arc};
use tokio::{
    join,
    net::UdpSocket,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex, RwLock,
    },
    task::JoinHandle,
};

const TRANS_ID: i32 = 10;

#[derive(Debug)]
pub enum TError {
    NoTrackerResolved,
}

#[derive(Debug)]
pub struct TorrentFile {
    /// Path of the torrent file
    pub path: String,

    /// DataStructure that holds metadata about the date encoded inside of ".torrent" file

    /// Stores the total no of pieces
    pub pieces_count: usize,

    /// The data that changes during runtime and gets observed by other entity to display the
    /// progress.
    ///
    /// For eg. A UI library is keeping track of the changes in "state" every 2 seconds and displaying the UI,
    /// here we are making changes in the state field continuosly
    pub state: Arc<State>,

    /// Trackers get the socket address of the peers.
    /// Let's say there are 20 trackers, we are communicating to,
    /// and when all of them have this UnboundedSender, then we can simply
    /// use this channel to collect the peers and after collecting the
    /// peers we can invoke run() method of the peer and
    /// store in the peers field of [Peers]
    peers_channel: (Arc<UnboundedSender<Peer>>, Arc<Mutex<UnboundedReceiver<Peer>>>),
}

struct Peers {
    peers: Vec<u32>,
}

/// TODO: Implement DHT(Distributed Hash Table) as well
impl TorrentFile {
    /// It will try to parse the given the path of the torrent file and create a new data structure
    /// from the Torrent file
    pub async fn new(path: &String) -> Option<Self> {
        match FileMeta::fromTorrentFile(&path) {
            Ok(meta_info) => {
                let info_hash = meta_info.generateInfoHash();
                let pieces_hash = meta_info.getPiecesHash();
                let pieces_count = pieces_hash.len();
                let d_state = DownState::Unknown;
                let file_tree = Some(Self::generateFileTree(&meta_info).await);
                let trackers = ArcRwLock!(Vec::new());
                let udp_ports = ArcMutex!(Vec::new());
                let tcp_ports = ArcMutex!(Vec::new());
                let peers = ArcMutex!(Vec::new());
                let bytes_complete = ACell!(3000000000);
                let pieces_downloaded = ACell!(120);
                let uptime = ACell!(0);

                let peers_channel = unbounded_channel::<Peer>();
                let peers_channel = (Arc::new(peers_channel.0), ArcMutex!(peers_channel.1));

                let state = Arc::new(State {
                    pieces_downloaded,
                    bytes_complete,
                    meta_info,
                    d_state,
                    file_tree,
                    trackers,
                    udp_ports,
                    tcp_ports,
                    info_hash,
                    pieces_hash,
                    peers,
                    uptime,
                });

                Some(Self {
                    path: path.to_string(),
                    pieces_count,
                    state,
                    peers_channel,
                })
            }
            _ => None,
        }
    }

    //// NOTE : This function is assumed to be called once in the download session
    ///// Creates objects of [Tracker] by extracting out all the Trackers from "announce" and "announce-list" field
    ///// and then resolves their address through DNS lookup
    //async fn resolveTrackers(&self) -> Result<(), TError> {
    //let trackers = self.state.trackers.clone();
    //let mut tracker_s: Vec<Vec<Arc<Tracker>>> = Vec::new(); // Stores the resolved trackers

    //// According to BEP12, if announce_list field is present then the client will have to
    //// ignore the announce field as the URL in the announce field is already present in the
    //// announce_list
    ////
    //// Inside this function resolveTrackers(...) only the initial step of extracting out the
    //// URLS from announce_list or announce fild is considered and resolving the DNS of the URL
    //// is done
    //if let Some(ref d) = self.state.meta_info.announce_list {
    //for i in d {
    //let x: Vec<Arc<Tracker>> = {
    //let mut trackers = vec![];
    //for addrs in i {
    //// This tries to parse the given URL and if it parses successfully then
    //// signals to resolve the socket address
    //let torrent_state = self.state.clone();
    //if let Ok(mut tracker) = Tracker::new(addrs, torrent_state, self.peers_channel.0.clone()) {
    //// TODO : Figure out what to do to those trackers who DNS wasn't
    //// resolved, whether to try after a certain time or what
    //if tracker.resolveSocketAddr() {
    //trackers.push(Arc::new(tracker));
    //}
    //}
    //}
    //trackers
    //};
    //tracker_s.push(x);
    //}
    //} else {
    //let ref addrs = self.state.meta_info.announce;
    //let torrent_state = self.state.clone();
    //if let Ok(mut tracker) = Tracker::new(addrs, torrent_state, self.peers_channel.0.clone()) {
    //if tracker.resolveSocketAddr() {
    //tracker_s.push(vec![Arc::new(tracker)]);
    //}
    //}
    //}

    //return if tracker_s.len() == 0 {
    //Err(TError::NoTrackerResolved)
    //} else {
    //*(trackers.write().await) = tracker_s;
    //return Ok(());
    //};
    //}

    pub async fn generateFileTree(meta: &FileMeta) -> Arc<Mutex<File>> {
        // We'll consider the root file to be named "."
        File::new(meta, &".".to_owned()).await.unwrap()
    }

    async fn getUDPSocket(&self) -> Arc<UdpSocket> {
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
                    //println!("{:?}", e.to_string());
                    port = port + 1;
                }
            }
        }
    }

    // Running of trackers is divided into two sub tasks
    // 1. Sending trackers requests
    // 2. Receiving trackers response
    //
    // One is, sending Trackers requests and other is
    // If there are 'n' no of trackers, then
    // In the first async task of 'req', it spawns 'n' no of tasks within itself, and these each task make a
    // request and for every response that comes in the socket, its handled by second async
    // task of 'res
    async fn runTrackers(&self, socket: Arc<UdpSocket>) {
        // Step 1 : Generate "Tracker" instance from all the tracker's URL in "announce" or
        // "announce_list" field of FileMeta and spawm a tokio task internally to call each tracker's run method
        let trackers: Vec<Vec<Arc<Tracker>>> = {
            let mut tracker_s = Vec::default();
            if let Some(ref announce_list_s) = self.state.meta_info.announce_list {
                for announce_list in announce_list_s {
                    let mut _trackers = Vec::new();
                    for announce_url in announce_list {
                        if let Ok(tracker) = Tracker::new(announce_url, self.state.clone(), self.peers_channel.0.clone()) {
                            let tracker = Arc::new(tracker);
                            let tracker_cloned = tracker.clone();
                            tokio::spawn(async move {
                                tracker_cloned.resolveTracker().await;
                            });
                            _trackers.push(tracker);
                        }
                    }
                    tracker_s.push(_trackers);
                }
            } else {
                let ref announce_url = self.state.meta_info.announce;
                if let Ok(tracker) = Tracker::new(announce_url, self.state.clone(), self.peers_channel.0.clone()) {
                    tracker_s.push(vec![Arc::new(tracker)])
                }
            }
            tracker_s
        };
        *self.state.trackers.write().await = trackers;

        // Step 2 : Recv by listening on the UDP socket and then find out for whom the message came for and give
        // back to that specific tracker the response messsage
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
                _ => {
                    // Error on receiving from the UDP Socket
                }
            }
        }
    }

    pub async fn runDownload(&self) {
        let ref peers_rcv = self.peers_channel.1;
        let mut peers_rcv = peers_rcv.lock().await;
        while let Some(peer) = peers_rcv.recv().await {
            //println!("--------------------------------------------",);
            //println!("{:?}", peer.socket_adr);
            //println!("--------------------------------------------",);
        }
        // TODO: Run in a loop, but never return anything
    }

    /// TODO : Add examples for the rust docs
    /// Starts to download the torrent, it will keep on mutating the "state" field as it
    /// makes progress, and if the torrent needs to be pause or started, one can use the method on
    /// that State instance
    ///
    /// NOTE : While using this method, one must clone and keep a Arc pointer of "state" field,
    /// so that they can use it later on to display the UI or the data changed
    pub async fn run(&self) {
        // A UDP socket for all the Trackers to send requests and receive responses
        let trackers_udp_socket = self.getUDPSocket().await;

        let run_trackers = self.runTrackers(trackers_udp_socket.clone());
        let run_download = self.runDownload();

        join!(run_trackers, run_download);
    }
}
