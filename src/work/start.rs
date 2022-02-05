// NOTE: When i say files, i mean it in Unix term. Folders are files as well, their type is Directory

use super::tracker::Tracker;
use super::{file, torrent_parser};
use crate::{ui::files::FilesState, work::file::File};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::net::UdpSocket;

// Starting Point for the working thread
pub fn start((app_state, torrent_file_path): (Arc<Mutex<FilesState>>, String)) {
    let (file_meta, info_hash) = torrent_parser::parse_file(&torrent_file_path);

    let async_block = async move {
        //        tracker_request(info_hash, trackers).await;
        //for tracker in &trackers {
        //   println!("{:?}", &tracker);
        //}
    };

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async_block);
}

// GOAL : Constantly polls the Tracker with UDP request after certain interval
// CURRENTLY : Blocks the thread
// TODO : Make this process of polling trackers non blocking, for now it's blocking for idk what
// reason
async fn tracker_request(info_hash: Vec<u8>, trackers: Vec<std::cell::RefCell<Tracker>>) {
    //
    const TRANS_ID: i32 = 10;
    const PORT: i16 = 8001;
    let socket_address: SocketAddr = format!("[::]:{}", PORT).parse().unwrap();
    let socket = UdpSocket::bind(socket_address).await.unwrap();
    let x = Instant::now();

    println!("{:?}", Instant::now().duration_since(x));

    //  for tracker in &trackers {
    //      let tracker_borrow = tracker.borrow();
    //      let x = std::time::Instant::now();
    //      if tracker_borrow.protocol == TrackerProtocol::UDP {
    //          // Create Connection Request
    //          if let Ok(addrs) = tracker_borrow.url.socket_addrs(|| None) {
    //              // Sets the socket address of the URL
    //              let mut tracker_borrow_mut = tracker.borrow_mut();
    //              tracker_borrow_mut.socket_adr = Some(addrs[0]);

    //              drop(tracker_borrow);
    //              match connect_request(TRANS_ID, &socket, &addrs[0], tracker).await {
    //                  Ok(connect_response) => {
    //                      if let Ok(_) = annnounce_request(
    //                          connect_response.clone(),
    //                          &socket,
    //                          &addrs[0],
    //                          info_hash.clone(),
    //                      )
    //                      .await
    //                      {
    //                          tracker.borrow_mut().didItResolve = true;
    //                      }
    //                  }
    //                  _ => {
    //                      println!("Error Time : {:?}", Instant::now().duration_since(x));
    //                  }
    //              }
    //          }
    //      }
    //  }
}
