// NOTE: When i say files, i mean it in Unix term. Folders are files as well, their type is Directory

use super::tracker::{annnounce_request, connect_request, AnnounceResponse, ConnectResponse, Tracker, TrackerProtocol};
use crate::details::Details;
use crate::ui::files::FilesState;
use futures::future::join_all;
use futures::TryFutureExt;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::join;
use tokio::net::{TcpSocket, TcpStream, UdpSocket};
use tokio::time::timeout;

// Starting Point for the working thread
pub fn start(file_state: Arc<Mutex<FilesState>>, trackers: Arc<Mutex<Vec<Arc<Mutex<RefCell<Tracker>>>>>>, details: Arc<Mutex<Details>>) {
    const CONNECT_REQUEST_UDP_SOCKET_PORT: i16 = 8001;
    const SCRAPE_REQUEST_UDP_SOCKET_PORT: i16 = 8002;
    const ANNOUNCE_REQUEST_UDP_SOCKET_PORT: i16 = 8003;
    const PEERS_TCP_SOCKET_PORT: i16 = 8004;

    let peers: Rc<RefCell<Vec<SocketAddr>>> = Rc::new(RefCell::new(Vec::new()));

    let info_hash = details.lock().unwrap().info_hash.clone().unwrap();

    let async_block = async move {
        let peers_tcp_socket_address: SocketAddr = format!("127.0.0.1:{}", PEERS_TCP_SOCKET_PORT).parse().unwrap();
        let peers_tcp_socket = Arc::new(TcpSocket::new_v4().unwrap());
        peers_tcp_socket.bind(peers_tcp_socket_address).unwrap();

        // UDP Socket to send Connect Request and receive Connect Response
        let connect_request_udp_socket_address: SocketAddr = format!("[::]:{}", CONNECT_REQUEST_UDP_SOCKET_PORT).parse().unwrap();
        let connect_request_udp_socket = UdpSocket::bind(connect_request_udp_socket_address).await.unwrap();

        // UDP Socket to send Announce Request and receive Announce Response
        let scrape_request_udp_socket_address: SocketAddr = format!("[::]:{}", SCRAPE_REQUEST_UDP_SOCKET_PORT).parse().unwrap();
        let scrape_request_udp_socket = UdpSocket::bind(scrape_request_udp_socket_address).await.unwrap();

        // UDP Socket to send Scrape Request and receive Scrape Response
        let annnounce_request_udp_socket_address: SocketAddr = format!("[::]:{}", ANNOUNCE_REQUEST_UDP_SOCKET_PORT).parse().unwrap();
        let annnounce_request_udp_socket = UdpSocket::bind(annnounce_request_udp_socket_address).await.unwrap();

        let trackers_request = trackers_request(
            trackers.clone(),
            &connect_request_udp_socket,
            &scrape_request_udp_socket,
            &annnounce_request_udp_socket,
            info_hash.clone(),
        );

        let mut v = Vec::new();
        for _ in 1..100 {
            let peers_request = peers_request(trackers.clone(), &peers_tcp_socket);
            v.push(peers_request);
        }

        join_all(v).await;

        //        join!(trackers_request, peers_request);
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

    //let trackers_lock = trackers.lock().unwrap();
    //let mut h = HashSet::new();
    //for tracker in &(*trackers_lock) {
    //let tracker_lock = tracker.lock().unwrap();
    // let tracker_borrow = tracker_lock.borrow();
    //  for socket in &tracker_borrow.announce_response.as_ref().unwrap().peersAddresses {
    //       h.insert(socket);
    //    }
    // }
}

//async fn peer_request(peers_tcp_socket: &TcpSocket, socket_adr: &SocketAddr) {}

// Polls all the trackers concurrently
async fn trackers_request(
    trackers: Arc<Mutex<Vec<Arc<Mutex<RefCell<Tracker>>>>>>,
    connect_request_udp_socket: &UdpSocket,
    scrape_request_udp_socket: &UdpSocket,
    annnounce_request_udp_socket: &UdpSocket,
    info_hash: Vec<u8>,
) {
    let trackers_lock = trackers.lock().unwrap();
    let mut futures: Vec<_> = vec![];
    for tracker in &(*trackers_lock) {
        let tracker_lock = tracker.lock().unwrap();
        let tracker_borrow = tracker_lock.borrow();
        if tracker_borrow.protocol == TrackerProtocol::UDP && tracker_borrow.socket_adr != None {
            futures.push(tracker_request(
                tracker.clone(),
                &connect_request_udp_socket,
                &scrape_request_udp_socket,
                &annnounce_request_udp_socket,
                info_hash.clone(),
            ));
        }
    }

    join_all(futures).await;
}

// Makes UDP request to a tracker in certain interval
async fn tracker_request(
    tracker: Arc<Mutex<RefCell<Tracker>>>,
    connect_request_udp_socket: &UdpSocket,
    scrape_request_udp_socket: &UdpSocket,
    annnounce_request_udp_socket: &UdpSocket,
    info_hash: Vec<u8>,
) {
    const TRANS_ID: i32 = 10;

    loop {
        let tracker_lock = tracker.lock().unwrap();
        let tracker_borrow = tracker_lock.borrow();
        let socket_adr = &tracker_borrow.socket_adr.unwrap();
        drop(tracker_borrow);
        drop(tracker_lock);

        // Make Connect Request to the tracker
        if let Ok(_) = connect_request(TRANS_ID, &connect_request_udp_socket, socket_adr, tracker.clone()).await {
            // If the request was sent successfully
            let mut buf = vec![0; 16];
            // Wait for 4 secs to receive something after sending Connect Request
            match timeout(Duration::from_secs(4), connect_request_udp_socket.recv_from(&mut buf)).await {
                Ok(_) => {
                    let connect_response = ConnectResponse::from_array_buffer(buf);
                    match annnounce_request(connect_response, scrape_request_udp_socket, socket_adr, info_hash.clone(), tracker.clone()).await {
                        Ok(_) => {
                            let mut response = vec![0; 1024];
                            match timeout(Duration::from_secs(4), scrape_request_udp_socket.recv_from(&mut response)).await {
                                Ok(x) => {
                                    let v = x.unwrap().0;
                                    //    println!("{:?}", response);
                                    response = response.drain(0..v).collect();
                                    //   println!("{:?}", response);
                                    if let Ok(resp) = AnnounceResponse::new(&response) {
                                        println!("{:?}", resp);
                                        tokio::time::sleep(Duration::from_secs(resp.interval as u64)).await;
                                    } else {
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                Err(_) => {
                    // Makes request to the tracker in every 5 sec
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            };
        };

        // Makes request to the tracker in every 5 sec
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
