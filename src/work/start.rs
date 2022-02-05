// NOTE: When i say files, i mean it in Unix term. Folders are files as well, their type is Directory

use super::tracker::annnounce_request;
use super::tracker::connect_request;
use super::tracker::Tracker;
use super::tracker::TrackerProtocol;
use crate::ui::files::FilesState;
use crate::work::tracker;
use crate::Details;
use futures::future;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::time::interval;

// Starting Point for the working thread
pub fn start(
    (_, _): (Arc<Mutex<FilesState>>, String),
    trackers: Arc<Mutex<Vec<RefCell<Tracker>>>>,
    details: Arc<Mutex<Details>>,
) {
    let info_hash = details.lock().unwrap().info_hash.clone().unwrap();
    let trackers_lock = trackers.lock().unwrap();
    let async_block = async move {
        const TRANS_ID: i32 = 10;
        const PORT: i16 = 8001;
        let socket_address: SocketAddr = format!("[::]:{}", PORT).parse().unwrap();
        let socket = UdpSocket::bind(socket_address).await.unwrap();
        let t = Instant::now();
        let mut v: Vec<_> = vec![];
        for tracker in &(*trackers_lock) {
            let tracker_borrow = tracker.borrow();
            if tracker_borrow.protocol == TrackerProtocol::UDP && tracker_borrow.socket_adr != None
            {
                v.push(get_udp_tracker_info(tracker, &socket));
            }
        }
        future::join_all(v).await;
    };

    async fn get_udp_tracker_info(tracker: &RefCell<Tracker>, socket: &UdpSocket) {
        const TRANS_ID: i32 = 10;
        let tracker_borrow = tracker.borrow();
        let socket_adr = &tracker_borrow.socket_adr.unwrap();
        drop(tracker_borrow);
        connect_request(TRANS_ID, &socket, socket_adr, tracker).await;
    }

    async fn YO() {
        tokio::time::sleep(Duration::from_secs(1)).await;
        println!("YO");
    }

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async_block);
}

// GOAL : Constantly polls the Tracker with UDP request after certain interval
// CURRENTLY : Blocks the thread
// TODO : Make this process of polling trackers non blocking, for now it's blocking for idk what
// reason
//async fn tracker_request(info_hash: Vec<u8>, trackers: Arc<Mutex<Vec<RefCell<Tracker>>>>) {
//    const TRANS_ID: i32 = 10;
//    const PORT: i16 = 8001;
//    let socket_address: SocketAddr = format!("[::]:{}", PORT).parse().unwrap();
//    let socket = UdpSocket::bind(socket_address).await.unwrap();
//    let y = Instant::now();
//    for tracker in &(*trackers.lock().unwrap()) {
//        let x = Instant::now();
//        let tracker_borrow = tracker.borrow();
//        if tracker_borrow.protocol == TrackerProtocol::UDP && tracker_borrow.socket_adr != None {
//            let socket_adr = &tracker_borrow.socket_adr.unwrap();
//            drop(tracker_borrow);
//            match connect_request(TRANS_ID, &socket, socket_adr, tracker).await {
//                Ok(connect_response) => {
//                    if let Ok(_) = annnounce_request(
//                        connect_response.clone(),
//                        &socket,
//                        socket_adr,
//                        info_hash.clone(),
//                    )
//                    .await
//                    {
//                        tracker.borrow_mut().didItResolve = true;
//                        println!("Resolve time : {:?}", Instant::now().duration_since(x));
//                    }
//                }
//                _ => {
//                    println!("Error Time : {:?}", Instant::now().duration_since(x));
//                }
//            }
//        }
//    }
//    println!("TOTAL TIME TAKEN : {:?}", Instant::now().duration_since(y));
//}
