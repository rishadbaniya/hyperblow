// This module handles everything related with a Tracker :
// The UDP Tracker Protocol is followed from : http://www.bittorrent.org/beps/bep_0015.html
// TODO : {
//    1 : Add Scrape Request
//    2 : Add TCP Tracker request
// }
//

#![allow(unused_must_use)]
#![allow(dead_code)]

use crate::Result;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use reqwest::Url;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex as TokioMutex;

const TRACKER_ERROR: &str = "There is something wrong with the torrent file you provided \n Couldn't parse one of the tracker URL";

// Offset          Size            Name            Value
// 0               64-bit integer  connection_id
// 8               32-bit integer  action          2 // scrape
// 12              32-bit integer  transaction_id
// 16 + 20 * n     20-byte string  info_hash
// 16 + 20 * N
struct ScrapeRequest {
    connection_id: Option<i64>,
    action: i32,
    transaction_id: Option<i32>,
    info_hash: Option<Vec<u8>>,
}

impl Default for ScrapeRequest {
    fn default() -> Self {
        Self {
            connection_id: None,
            action: 2,
            transaction_id: None,
            info_hash: None,
        }
    }
}

impl ScrapeRequest {
    // Generates a BytesMut by consuming field of ScrapeRequest
    fn getBytesMut(&self) -> BytesMut {
        let mut bytes = BytesMut::with_capacity(36);
        bytes.put_i64(self.connection_id.unwrap());
        bytes.put_i32(self.action);
        bytes.put_i32(self.transaction_id.unwrap());
        bytes.put_slice(self.info_hash.as_ref().unwrap().as_slice());
        bytes
    }

    fn set_connection_id(&mut self, v: i64) {
        self.connection_id = Some(v);
    }

    fn set_transaction_id(&mut self, v: i32) {
        self.transaction_id = Some(v);
    }

    // NOTE : It doesnt explicity takes a info hash of 20 bytes
    // TODO : Make sure the input is 20 bytes so that any sort of bug doesn't occur
    fn set_info_hash(&mut self, v: Vec<u8>) {
        self.info_hash = Some(v);
    }
}

// Offset     Size            Name            Value
// 0           32-bit integer  action          2 // scrape
// 4           32-bit integer  transaction_id
// 8 + 12 * n  32-bit integer  seeders
// 12 + 12 * n 32-bit integer  completed
// 16 + 12 * n 32-bit integer  leechers
// 8 + 12 * N

struct ScrapeReponse {
    action: i32,
    transaction_id: i32,
    seeders: i32,
}

impl Tracker {
    /// Create a new Tracker from given Url String
    pub fn new(url: &String) -> Self {
        let url = Url::parse(url).expect(TRACKER_ERROR);
        let protocol = {
            if url.scheme() == "udp" {
                TrackerProtocol::UDP
            } else {
                TrackerProtocol::HTTP
            }
        };
        Tracker {
            url,
            protocol,
            socket_adr: None,
            didItResolve: false,
            connect_request: None,
            connect_response: None,
            announce_request: None,
            announce_response: None,
        }
    }

    /// Create list of "Tracker" from data in the announce and announce_list field of "FileMeta"
    pub fn getTrackers(announce: &String, announce_list: &Vec<Vec<String>>) -> Vec<Arc<TokioMutex<Tracker>>> {
        let mut trackers: Vec<_> = Vec::new();

        // TODO : Find difference between Announce and Announce List coz i found Announce duplicate
        // in Announce List
        //trackers.push(Arc::new(sync::Mutex::new(RefCell::new(Tracker::new(announce)))));

        for tracker_url in announce_list {
            trackers.push(Arc::new(TokioMutex::new(Tracker::new(&tracker_url[0]))));
        }
        trackers
    }
}

// To be called at the first step of communicating with the UDP Tracker Servera
pub async fn connect_request(transaction_id: i32, socket: &UdpSocket, to: &SocketAddr, tracker: Arc<TokioMutex<Tracker>>) -> Result<()> {
    let mut guard_tracker = tracker.lock().await;
    let mut connect_request = ConnectRequest::empty();
    connect_request.set_transaction_id(transaction_id);
    guard_tracker.connect_request = Some(connect_request.clone());
    let buf = connect_request.getBytesMut();
    socket.send_to(&buf, to).await?;
    guard_tracker.connect_request = Some(connect_request);
    Ok(())
}

// To be called after having an instance of "ConnectResponse" which can be obtained
// after making a call to "connect_request"
pub async fn annnounce_request(
    connect_response: ConnectResponse,
    socket: &UdpSocket,
    to: &SocketAddr,
    details: Arc<TokioMutex<Details>>,
    tracker: Arc<TokioMutex<Tracker>>,
) -> Result<()> {
    // NOTE : The message received after sending "Announce Request" is kinda dynamic in a sense that
    // it has unknown amount of peers ip addresses and ports
    // Buffer to store the response

    let lock_details = details.lock().await;
    let mut announce_request = AnnounceRequest::empty();
    announce_request.set_connection_id(connect_response.connection_id);
    announce_request.set_transaction_id(connect_response.transaction_id);
    announce_request.set_info_hash(lock_details.info_hash.as_ref().unwrap().clone().to_vec());
    announce_request.set_downloaded(0);
    announce_request.set_uploaded(0);
    announce_request.set_uploaded(0);
    announce_request.set_left(lock_details.total_bytes);
    announce_request.set_port(8001);
    announce_request.set_key(20);
    socket.send_to(&announce_request.getBytesMut(), to).await?;

    let mut lock_tracker = tracker.lock().await;
    lock_tracker.announce_request = Some(announce_request);
    //let (_, _) = timeout(Duration::from_secs(4), socket.recv_from(&mut response)).await??;
    //let announce_response = AnnounceResponse::new(&response);
    Ok(())
}

pub async fn scrape_request(
    connection_response: ConnectResponse,
    socket: &UdpSocket,
    to: &SocketAddr,
    info_hash: Vec<u8>,
    tracker: Arc<Mutex<RefCell<Tracker>>>,
) -> Result<()> {
    // TODO : Put ScrapeRequest instance inside of Tracker Instance
    //let tracker_lock = tracker.lock().unwrap();
    //let mut tracker_borrow_mut = tracker_lock.borrow_mut();
    //
    let mut scrape_request = ScrapeRequest::default();
    scrape_request.set_connection_id(connection_response.connection_id);
    scrape_request.set_transaction_id(connection_response.transaction_id);
    scrape_request.set_info_hash(info_hash.clone());

    socket.send_to(&scrape_request.getBytesMut(), to).await?;
    Ok(())
}
