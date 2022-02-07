// NOTE: When i say files, i mean it in Unix term. Folders are files as well, their type is Directory

use super::tracker::{self, annnounce_request, connect_request, AnnounceResponse, ConnectResponse, Tracker, TrackerProtocol};
use crate::details::Details;
use crate::ui::files::FilesState;
use byteorder::{BigEndian, ReadBytesExt};
use futures::future::join_all;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::join;
use tokio::net::{TcpSocket, TcpStream, UdpSocket};
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::{mpsc, mpsc::Receiver};
use tokio::time::timeout;
use tracker::udp_socket_recv;

// Starting Point for the working thread
pub fn start(file_state: Arc<Mutex<FilesState>>, trackers: Arc<TokioMutex<Vec<Arc<TokioMutex<RefCell<Tracker>>>>>>, details: Arc<Mutex<Details>>) {
    const UDP_SOCKET_PORT: i16 = 8001;
    //const PEERS_TCP_SOCKET_PORT: i16 = 8004;

    let peers: Rc<RefCell<Vec<SocketAddr>>> = Rc::new(RefCell::new(Vec::new()));
    let info_hash = details.lock().unwrap().info_hash.clone().unwrap();

    let async_block = async move {
        let udp_socket_addr: SocketAddr = format!("[::]:{}", UDP_SOCKET_PORT).parse().unwrap();
        let udp_socket = UdpSocket::bind(udp_socket_addr).await.unwrap();

        // Channel to communicate  between the UDP socket recv end and Trackers end
        let (trackers_sender, trackers_receiver) = {
            let mut senders = Vec::new();
            let mut receivers = Vec::new();
            let trackers_lock = trackers.lock().await;

            for _ in trackers_lock.iter() {
                let (send, recv) = mpsc::channel::<Vec<u8>>(2048);
                senders.push(send);
                receivers.push(Rc::new(RefCell::new(recv)));
            }
            (senders, receivers)
        };
        let udp_socket_send = trackers_request(trackers.clone(), &udp_socket, info_hash.clone(), trackers_receiver, trackers_sender);

        udp_socket_send.await;
        println!("{:?}", trackers);
    };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap()
        .block_on(async_block);
}

// Polls all the peers concurrently
async fn peers_request(trackers: Arc<Mutex<Vec<Arc<Mutex<RefCell<Tracker>>>>>>, peers_tcp_socket: &TcpSocket) {
    loop {
        let t = Instant::now();
        let xx: SocketAddr = "142.250.74.238:80".parse().unwrap();
        let x = TcpStream::connect(xx).await.unwrap();
        println!("DID IT WORK {:?} {:?}", x, Instant::now().duration_since(t));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

// Polls all the trackers concurrently
async fn trackers_request(
    trackers: Arc<TokioMutex<Vec<Arc<TokioMutex<RefCell<Tracker>>>>>>,
    udp_socket: &UdpSocket,
    info_hash: Vec<u8>,
    receivers: Vec<Rc<RefCell<Receiver<Vec<u8>>>>>,
    senders: Vec<Sender<Vec<u8>>>,
) {
    let trackers_lock = trackers.lock().await;
    let mut futures: Vec<_> = vec![];
    for (index, tracker) in (*trackers_lock).iter().enumerate() {
        let tracker_lock = tracker.lock().await;
        let tracker_borrow = tracker_lock.borrow();
        if tracker_borrow.protocol == TrackerProtocol::UDP && tracker_borrow.socket_adr != None {
            futures.push(tracker_request(tracker.clone(), udp_socket, info_hash.clone(), receivers[index].clone()));
        }
    }
    drop(trackers_lock);
    join!(join_all(futures), udp_socket_recv(&udp_socket, senders, trackers.clone()));
}

// Makes UDP request to a tracker in Certain Interval
// 1. Sends a Connect Request to the Tracker
// If this connect request arrives within
async fn tracker_request(tracker: Arc<TokioMutex<RefCell<Tracker>>>, udp_socket: &UdpSocket, info_hash: Vec<u8>, receiver: Rc<RefCell<Receiver<Vec<u8>>>>) {
    const TRANS_ID: i32 = 10;
    const CONNECT_REQUEST_TIMEOUT: u64 = 4;
    const SENDING_A_CONNECT_REQUEST_FAILED__RETRY_AFTER: u64 = 100;
    const ANNOUNCE_REQUEST_TIMEOUT: u64 = 4;
    const SENDING_A_ANNOUNCE_REQUEST_FAILED__RETRY_AFTER: u64 = 2;

    loop {
        let tracker_lock = tracker.lock().await;
        let tracker_borrow = tracker_lock.borrow();
        let socket_adr = &tracker_borrow.socket_adr.unwrap();
        drop(tracker_borrow);
        drop(tracker_lock);

        let mut receiver_borrow_mut = receiver.borrow_mut();

        // Make Connect Request to the tracker
        if let Ok(_) = connect_request(TRANS_ID, &udp_socket, socket_adr, tracker.clone()).await {
            match timeout(Duration::from_secs(CONNECT_REQUEST_TIMEOUT), receiver_borrow_mut.recv()).await {
                Ok(v) => {
                    let buf = v.unwrap();
                    let mut action_bytes = &buf[0..=3];
                    let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes).unwrap();
                    match action {
                        1 => {
                            println!("{:?}", buf);
                        }
                        _ => {}
                    }
                }
                Err(_) => {}
            };
        } else {
            tokio::time::sleep(Duration::from_secs(SENDING_A_CONNECT_REQUEST_FAILED__RETRY_AFTER)).await;
        };
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
