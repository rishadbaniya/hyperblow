use super::tracker::{udp_socket_recv, Tracker, TrackerProtocol};
use crate::details::Details;
use crate::ui::files::FilesState;
use crate::work::tracker::tracker_request;
use futures::future::join_all;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::Arc;

use super::peer::peer_request;
use tokio::join;
use tokio::net::UdpSocket;
use tokio::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
    Mutex,
};
use tokio::task;

pub type __Trackers = Arc<Mutex<Vec<Arc<Mutex<Tracker>>>>>;
pub type __Details = Arc<Mutex<Details>>;
pub type __FileState = Arc<Mutex<FilesState>>;

// Starting Point for the "Working" thread
pub fn start(file_state: __FileState, trackers: __Trackers, details: __Details) {
    //
    // UDP Socket to send and receive messages
    // Send => Connect Request, Scrape Request, Announce Request
    // Receive => Connect Response, Scrape Response, Announce Response
    let UDP_SOCKET: SocketAddr = "[::]:8002".parse().unwrap();

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

        let peers_tcp_stream = peers_request(trackers.clone(), receiver_peers, details.clone());
        tokio::join!(udp_socket_send, peers_tcp_stream);
    };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap()
        .block_on(async_block);
}

/// NOTE: This async function must be run concurrently using join!{}
/// It constantly downloads "pieces" from peers concurrently
async fn peers_request(trackers: __Trackers, peers_receiver: RefCell<Receiver<Vec<SocketAddr>>>, details: __Details) {
    let mut peers_receiver = peers_receiver.borrow_mut();

    // Stores all the Socket Addresses of the Peer
    let mut peers = Vec::new();
    loop {
        // Receives Socket Address sent by Tracker
        if let Some(socket_addresses) = peers_receiver.recv().await {
            // Stores peer Socket Address that was not received previously
            let mut newly_added_peers = Vec::new();
            for socket_addr in socket_addresses {
                if !peers.contains(&socket_addr) {
                    peers.push(socket_addr);
                    newly_added_peers.push(socket_addr);
                }
            }
            if !newly_added_peers.is_empty() {
                for socket_adr in newly_added_peers {
                    let _details = details.clone();

                    task::spawn(async move { peer_request(socket_adr, _details).await });
                }
            }
        }
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
        let tracker_guard = tracker.lock().await;
        if tracker_guard.protocol == TrackerProtocol::UDP && tracker_guard.socket_adr != None {
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
