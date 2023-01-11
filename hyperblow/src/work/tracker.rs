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

//
// It Constantly listens on the UDP socket for all the message and
// after receiving the response, it communicates with the specific "Tracker" using "Channel" by
// sending the received response through "Sender"
//
// By all the responses i mean "Connect Response" "Announce Response" and "Scrape Reponse".
// It will automatically figure out which response is which and for which Tracker
//
// Figuring out which response is for which Tracker :
// Here, all the socket address of Trackers are taken in their respective index and kept in a vec
// called "socket_adresses" in "String" form, so when some UDP response comes from far way Server,
// address from where the message came is taken in "String" form and the index of the
// socket address is found by tallying it against "socket_adresses", this index is same as the index in which
// Tracker is kept in trackers and Sender for Tracker in "senders"
//
// If the action != 3 i.e error from the server, then the message is forwaded through the Channel
// to the respective Tracker through "Sender"
//

pub async fn udp_socket_recv(udp_socket: &UdpSocket, senders: Vec<Sender<Vec<u8>>>, trackers: Arc<TokioMutex<Vec<Arc<TokioMutex<Tracker>>>>>) {
    let socket_adresses = {
        let trackers_lock = trackers.lock().await;
        let mut socket_adresses = Vec::new();
        for tracker in &(*trackers_lock) {
            if let Some(s) = &tracker.lock().await.socket_adr {
                socket_adresses.push(format!("{}:{}", s.ip(), s.port()));
            } else {
                socket_adresses.push(String::from(""))
            }
        }
        socket_adresses
    };

    loop {
        let mut buf = vec![0; 1024];
        match udp_socket.recv_from(&mut buf).await {
            Ok(v) => {
                //    println!("{:?}", v.1);
                let size = v.0;
                let buf = buf.drain(0..v.0).collect::<Vec<u8>>();
                let socket_adr = v.1;
                let ip = format!("{}", socket_adr.ip()).replace(":", "").replace("f", "");
                let port = socket_adr.port();
                let socket_adr = format!("{}:{}", ip, port);
                for (i, v) in socket_adresses.iter().enumerate() {
                    if *v == socket_adr {
                        let mut action_bytes = &buf[0..=3];
                        let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes).unwrap();
                        if action != 3 {
                            senders[i].send(buf).await;
                        }
                        break;
                    }
                }
            }
            _ => {}
        }
    }
}

use crate::details::Details;
use std::rc::Rc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time::{sleep, timeout};
// Makes UDP request to a tracker in Certain Interval
// 1. Sends a Connect Request to the Tracker
// If this connect request arrives within
pub async fn tracker_request(
    tracker: Arc<TokioMutex<Tracker>>,
    udp_socket: &UdpSocket,
    details: Arc<TokioMutex<Details>>,
    receiver: Rc<RefCell<Receiver<Vec<u8>>>>,
    peers_sender: Sender<Vec<SocketAddr>>,
) {
    const TRANS_ID: i32 = 10;
    let mut no_of_times_connect_request_timeout: u64 = 0;

    loop {
        let tracker_lock = tracker.lock().await;
        let socket_adr = tracker_lock.socket_adr.unwrap();
        drop(tracker_lock);
        let mut receiver_borrow_mut = receiver.borrow_mut();
        if let Ok(_) = connect_request(TRANS_ID, &udp_socket, &socket_adr, tracker.clone()).await {
            //
            // Waits for 15 * 2 ^ n seconds, where n is from 0 to 8 => (3840 seconds), for Connect Response to come
            //
            match timeout(Duration::from_secs(15 + 2 ^ no_of_times_connect_request_timeout), receiver_borrow_mut.recv()).await {
                //
                Ok(v) => {
                    no_of_times_connect_request_timeout = 0;
                    let buf = v.unwrap();
                    let mut action_bytes = &buf[0..=3];
                    let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes).unwrap();

                    // Action = 0 means it's "Connect Response"
                    if action == 0 {
                        let connect_response = ConnectResponse::from(buf);
                        if let Ok(_) = annnounce_request(connect_response, &udp_socket, &socket_adr, details.clone(), tracker.clone()).await {}
                        match timeout(Duration::from_secs(10), receiver_borrow_mut.recv()).await {
                            Ok(v) => {
                                let announce_response = AnnounceResponse::new(v.as_ref().unwrap()).unwrap();
                                peers_sender.send(announce_response.peersAddresses).await.unwrap();
                                sleep(Duration::from_secs(announce_response.interval as u64)).await;
                            }
                            _ => {}
                        }
                    }
                }
                Err(_) => {
                    no_of_times_connect_request_timeout += 1;
                }
            };
        } else {
            // This else portion runs when a Tracker Socket cant be reached, in this condition we
            // will poll the tracker again after 15 * 2 ^ n
            //
            // Waits for  seconds, where n is from 0 to 8 => (3840 seconds), for Connect Response to come
            // println!("Some error");
            sleep(Duration::from_secs(14 + 2 ^ no_of_times_connect_request_timeout)).await;
            if !no_of_times_connect_request_timeout == 8 {
                no_of_times_connect_request_timeout += 1;
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}
