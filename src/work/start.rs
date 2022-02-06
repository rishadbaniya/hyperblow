// NOTE: When i say files, i mean it in Unix term. Folders are files as well, their type is Directory

use super::tracker::connect_request;
use super::tracker::Tracker;
use super::tracker::TrackerProtocol;
use crate::details::Details;
use crate::ui::files::FilesState;
use crate::work::tracker::scrape_request;
use crate::work::tracker::ConnectResponse;
use futures::future::join_all;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::join;
use tokio::net::UdpSocket;

// Starting Point for the working thread
pub fn start(file_state: Arc<Mutex<FilesState>>, trackers: Arc<Mutex<Vec<Arc<Mutex<RefCell<Tracker>>>>>>, details: Arc<Mutex<Details>>) {
    const CONNECT_REQUEST_UDP_SOCKET_PORT: i16 = 8001;
    const SCRAPE_REQUEST_UDP_SOCKET_PORT: i16 = 8002;
    const ANNOUNCE_REQUEST_UDP_SOCKET_PORT: i16 = 8003;

    let info_hash = details.lock().unwrap().info_hash.clone().unwrap();

    let async_block = async move {
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
            trackers,
            &connect_request_udp_socket,
            &scrape_request_udp_socket,
            &annnounce_request_udp_socket,
            info_hash.clone(),
        );

        join!(trackers_request);
    };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap()
        .block_on(async_block);
}

use std::time::Duration;
use tokio::time::timeout;

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
                    match scrape_request(connect_response, scrape_request_udp_socket, socket_adr, info_hash.clone(), tracker.clone()).await {
                        Ok(_) => {
                            let mut response = vec![0; 1024];
                            match timeout(Duration::from_secs(4), scrape_request_udp_socket.recv_from(&mut response)).await {
                                Ok(x) => {
                                    let v = x.unwrap().0;
                                    response = response.drain(0..v).collect();
                                    println!("{:?}", response);
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
