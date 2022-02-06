// NOTE: When i say files, i mean it in Unix term. Folders are files as well, their type is Directory

use super::tracker::connect_request;
use super::tracker::Tracker;
use super::tracker::TrackerProtocol;
use crate::ui::files::FilesState;
use crate::work::tracker::annnounce_request;
use crate::work::tracker::AnnounceResponse;
use crate::work::tracker::ConnectResponse;
use crate::Details;
use futures::future;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::join;
use tokio::net::UdpSocket;

// Starting Point for the working thread
pub fn start(
    (_, _): (Arc<Mutex<FilesState>>, String),
    trackers: Arc<Mutex<Vec<RefCell<Tracker>>>>,
    details: Arc<Mutex<Details>>,
) {
    let info_hash = details.lock().unwrap().info_hash.clone().unwrap();
    let trackers_lock = trackers.lock().unwrap();
    let async_block = async move {
        const PORT: i16 = 8001;
        let socket_address: SocketAddr = format!("[::]:{}", PORT).parse().unwrap();
        let socket = UdpSocket::bind(socket_address).await.unwrap();
        let mut v: Vec<_> = vec![];
        for tracker in &(*trackers_lock) {
            let tracker_borrow = tracker.borrow();
            if tracker_borrow.protocol == TrackerProtocol::UDP && tracker_borrow.socket_adr != None
            {
                v.push(tracker_request(tracker, &socket, info_hash.clone()));
            }
        }
        join!(future::join_all(v));

        //udp_tracker_recv(&socket).await;
    };

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async_block);
}

use std::time::Duration;
use tokio::time::timeout;

// Makes UDP request to a tracker in certain interval
async fn tracker_request(tracker: &RefCell<Tracker>, socket: &UdpSocket, info_hash: Vec<u8>) {
    const TRANS_ID: i32 = 10;

    loop {
        let tracker_borrow = tracker.borrow();
        let socket_adr = &tracker_borrow.socket_adr.unwrap();
        drop(tracker_borrow);

        // Make Connect Request to the tracker
        if let Ok(_) = connect_request(TRANS_ID, &socket, socket_adr, tracker).await {
            // If the request was sent successfully
            let mut buf = vec![0; 16];
            // Wait for 4 secs to receive something after sending Connect Request
            match timeout(Duration::from_secs(4), socket.recv_from(&mut buf)).await {
                Ok(_) => {
                    let connect_response = ConnectResponse::from_array_buffer(buf);
                    match annnounce_request(
                        connect_response,
                        socket,
                        socket_adr,
                        info_hash.clone(),
                        tracker,
                    )
                    .await
                    {
                        Ok(_) => {
                            let mut response = vec![0; 1024];
                            match timeout(Duration::from_secs(4), socket.recv_from(&mut response))
                                .await
                            {
                                Ok(x) => {
                                    let v = x.unwrap().0;
                                    let mut response = response.drain(0..v).collect();
                                    if v >= 20 {
                                        let annnounce_response = AnnounceResponse::new(&response);
                                        println!("Size : {:?} and {:?}", v, annnounce_response);
                                        tokio::time::sleep(Duration::from_secs(
                                            (annnounce_response.interval - 10) as u64,
                                        ))
                                        .await;
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
        let tracker_borrow = tracker.borrow();

        // Makes request to the tracker in every 5 sec
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
