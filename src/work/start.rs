use super::tracker::{udp_socket_recv, Tracker, TrackerProtocol};
use crate::details::Details;
use crate::ui::files::FilesState;
use crate::work::tracker::tracker_request;
use futures::future::join_all;
use std::cell::RefCell;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tokio::join;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::{mpsc, mpsc::Receiver};

type __Trackers = Arc<TokioMutex<Vec<Arc<TokioMutex<RefCell<Tracker>>>>>>;
type __Details = Arc<TokioMutex<Details>>;
type __FileState = Arc<Mutex<FilesState>>;

// Starting Point for the "Working" thread
pub fn start(file_state: __FileState, trackers: __Trackers, details: __Details) {
    // UDP Socket to send and receive messages
    // Send => Connect Request, Scrape Request, Announce Request
    // Receive => Connect Response, Scrape Response, Announce Response
    //
    let UDP_SOCKET: SocketAddr = "[::]:8001".parse().unwrap();

    let async_block = async move {
        let udp_socket = UdpSocket::bind(UDP_SOCKET).await.unwrap();

        // Channel to send incoming UDP response to specific Tracker
        let (senders_trackers, receivers_trackers) = {
            let mut senders = Vec::new();
            let mut receivers = Vec::new();
            let trackers_lock = trackers.lock().await;

            for _ in trackers_lock.iter() {
                let (sd, rv) = mpsc::channel::<Vec<u8>>(2048);
                senders.push(sd);
                receivers.push(Rc::new(RefCell::new(rv)));
            }
            (senders, receivers)
        };

        // Channel to send peers "Socket Address" received from Tracker's Announce Response to them
        let (sender_peers, receiver_peers) = {
            let (sd, rv) = mpsc::channel::<Vec<SocketAddr>>(2048);
            let rv = RefCell::new(rv);
            (sd, rv)
        };

        let udp_socket_send = trackers_request(
            trackers.clone(),
            &udp_socket,
            receivers_trackers,
            senders_trackers,
            sender_peers,
            details.clone(),
        );

        let peers_tcp_stream = peers_request(trackers.clone(), receiver_peers);
        tokio::join!(udp_socket_send, peers_tcp_stream);
    };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap()
        .block_on(async_block);
}

// NOTE: It must be run concurrenlty using join!{}
// Polls all the peers concurrently
async fn peers_request(trackers: __Trackers, peers_receiver: RefCell<Receiver<Vec<SocketAddr>>>) {
    let mut peers_receiver = peers_receiver.borrow_mut();
    let mut peers = HashSet::new();
    loop {
        if let Some(v) = peers_receiver.recv().await {
            for socket_addrs in v {
                peers.insert(socket_addrs);
            }
        }
        println!("{:?}", peers.len());
    }
}

// NOTE: It must be run concurrenlty using join!{}
// It constantly polls all the trackers concurrently
async fn trackers_request(
    trackers: __Trackers,
    udp_socket: &UdpSocket,
    trackers_receivers: Vec<Rc<RefCell<Receiver<Vec<u8>>>>>,
    trackers_senders: Vec<Sender<Vec<u8>>>,
    peers_sender: Sender<Vec<SocketAddr>>,
    details: __Details,
) {
    let lock_trackers = trackers.lock().await;
    let mut futures: Vec<_> = vec![];

    for (index, tracker) in (*lock_trackers).iter().enumerate() {
        let tracker_lock = tracker.lock().await;
        let tracker_borrow = tracker_lock.borrow();
        if tracker_borrow.protocol == TrackerProtocol::UDP && tracker_borrow.socket_adr != None {
            futures.push(tracker_request(
                tracker.clone(),
                udp_socket,
                details.clone(),
                trackers_receivers[index].clone(),
                peers_sender.clone(),
            ));
        } else {
            // TODO : Add TCP trackers too
        }
    }

    drop(lock_trackers);

    join! {
        join_all(futures),
        udp_socket_recv(&udp_socket, trackers_senders, trackers.clone())
    };
}
