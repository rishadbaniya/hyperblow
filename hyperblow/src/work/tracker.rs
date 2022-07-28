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

/// Struct to handle the message to be sent to "Connect" on the UDP Tracker
/// Used to create a "16 byte" buffer to make "Connect Request"
///
/// Connect Request Bytes Structure:
///
/// Offset  Size            Name            Value
/// 0       64-bit integer  protocol_id     0x41727101980 // magic constant
/// 8       32-bit integer  action          0 // connect
/// 12      32-bit integer  transaction_id
/// 16
///
#[derive(Debug, Clone)]
pub struct ConnectRequest {
    pub protocol_id: i64,
    pub action: i32,
    pub transaction_id: Option<i32>,
}

impl ConnectRequest {
    pub fn empty() -> Self {
        Self {
            protocol_id: 0x41727101980,
            action: 0,
            transaction_id: None,
        }
    }

    pub fn set_transaction_id(&mut self, v: i32) {
        self.transaction_id = Some(v);
    }

    pub fn getBytesMut(&self) -> BytesMut {
        let mut bytes = BytesMut::with_capacity(16);
        bytes.put_i64(self.protocol_id);
        bytes.put_i32(self.action);
        bytes.put_i32(self.transaction_id.unwrap());
        bytes
    }
}

/// Struct to handle response message from "Connect" request to the UDP Tracker
/// Used to create an instance of AnnounceRequest
/// Connect Response Bytes Structure from the UDP Tracker Protocol :
///
/// Offset  Size            Name            Value
/// 0       32-bit integer  action          0 // connect
/// 4       32-bit integer  transaction_id
/// 8       64-bit integer  connection_id
/// 16
#[derive(Debug, Clone)]
pub struct ConnectResponse {
    pub action: i32,
    pub transaction_id: i32,
    pub connection_id: i64,
}

impl ConnectResponse {
    pub fn from(v: Vec<u8>) -> Self {
        let mut action_bytes = &v[0..=3];
        let mut transaction_id_bytes = &v[4..=7];
        let mut connection_id_bytes = &v[8..=15];
        let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes).unwrap();
        let transaction_id = ReadBytesExt::read_i32::<BigEndian>(&mut transaction_id_bytes).unwrap();
        let connection_id = ReadBytesExt::read_i64::<BigEndian>(&mut connection_id_bytes).unwrap();
        Self {
            action,
            transaction_id,
            connection_id,
        }
    }
}

/// Struct to handle "Announce" request message
/// Used to create a "98 byte" buffer to make "Announce Request"
/// Reference : http://www.bittorrent.org/beps/bep_0015.html
///
/// IPv4 announce request Bytes Structure:
/// Offset  Size    Name    Value
/// 0       64-bit integer  connection_id   The connection id acquired from establishing the connection.
/// 8       32-bit integer  action          Action. in this case, 1 for announce. See : https://www.rasterbar.com/products/libtorrent/udp_tracker_protocol.html#actions
/// 12      32-bit integer  transaction_id  Randomized by client
/// 16      20-byte string  info_hash       The info-hash of the torrent you want announce yourself in.
/// 36      20-byte string  peer_id         Your peer id. (Peer ID Convention : https://www.bittorrent.org/beps/bep_0020.html)
/// 56      64-bit integer  downloaded      The number of byte you've downloaded in this session.
/// 64      64-bit integer  left            The number of bytes you have left to download until you're finished.
/// 72      64-bit integer  uploaded        The number of bytes you have uploaded in this session.
/// 80      32-bit integer  event           0 // 0: none; 1: completed; 2: started; 3: stopped
/// 84      32-bit integer  IP address      Your ip address. Set to 0 if you want the tracker to use the sender of this UDP packet.u
/// 88      32-bit integer  key             A unique key that is randomized by the client.
/// 92      32-bit integer  num_want        The maximum number of peers you want in the reply. Use -1 for default.
/// 96      16-bit integer  port            The port you're listening on.
/// 98
#[derive(Debug, Clone)]
pub struct AnnounceRequest {
    connection_id: Option<i64>,
    action: i32,
    transaction_id: Option<i32>,
    info_hash: Option<Vec<u8>>,
    peer_id: Option<[u8; 20]>,
    downloaded: Option<i64>,
    left: Option<i64>,
    uploaded: Option<i64>,
    event: Option<i32>,
    ip_address: Option<i32>,
    key: Option<i32>,
    num_want: i32,
    port: Option<i16>,
}

impl AnnounceRequest {
    // Creates an empty Announce instance
    pub fn empty() -> Self {
        let peer_id_slice = b"-HYBxxx-yyyyyyyyyyyy";
        let mut peer_id = [0u8; 20];
        for (index, value) in peer_id_slice.iter().enumerate() {
            peer_id[index] = *value;
        }
        AnnounceRequest {
            connection_id: None,
            action: 1,
            transaction_id: None,
            info_hash: None,
            peer_id: Some(peer_id),
            downloaded: None,
            left: None,
            uploaded: None,
            event: Some(1),
            ip_address: Some(0),
            key: None,
            num_want: -1,
            port: None,
        }
    }

    // Consumes the Announce instance and gives you a Buffer of 98 bytes that you
    // can use to make Announce Request in UDP
    pub fn getBytesMut(&self) -> BytesMut {
        let mut bytes = BytesMut::with_capacity(98);
        bytes.put_i64(self.connection_id.unwrap());
        bytes.put_i32(self.action);
        bytes.put_i32(self.transaction_id.unwrap());
        bytes.put_slice(&self.info_hash.as_ref().unwrap()[..]);
        bytes.put_slice(&self.peer_id.unwrap());
        bytes.put_i64(self.downloaded.unwrap());
        bytes.put_i64(self.left.unwrap());
        bytes.put_i64(self.uploaded.unwrap());
        bytes.put_i32(self.event.unwrap());
        bytes.put_i32(self.ip_address.unwrap());
        bytes.put_i32(self.key.unwrap());
        bytes.put_i32(self.num_want);
        bytes.put_i16(self.port.unwrap());
        bytes
    }

    pub fn set_connection_id(&mut self, v: i64) {
        self.connection_id = Some(v);
    }

    pub fn set_transaction_id(&mut self, v: i32) {
        self.transaction_id = Some(v);
    }

    pub fn set_info_hash(&mut self, v: Vec<u8>) {
        self.info_hash = Some(v);
    }

    pub fn set_downloaded(&mut self, v: i64) {
        self.downloaded = Some(v);
    }

    pub fn set_uploaded(&mut self, v: i64) {
        self.uploaded = Some(v);
    }

    pub fn set_left(&mut self, v: i64) {
        self.left = Some(v);
    }

    pub fn set_port(&mut self, v: i16) {
        self.port = Some(v);
    }

    pub fn set_key(&mut self, v: i32) {
        self.key = Some(v);
    }
}

// IPv4 announce response:
//
// Offet      Size            Name            Value
// 0           32-bit integer  action          1 // announce
// 4           32-bit integer  transaction_id
// 8           32-bit integer  interval
// 12          32-bit integer  leechers
// 16          32-bit integer  seeders
// 20 + 6 * n  32-bit integer  IP address
// 24 + 6 * n  16-bit integer  TCP port
// 20 + 6 * Ns
//
// Struct to handle the response received by sending "Announce" request
#[derive(Debug, Clone)]
pub struct AnnounceResponse {
    pub action: i32,
    pub transaction_id: i32,
    pub interval: i32,
    pub leechers: i32,
    pub seeders: i32,
    pub peersAddresses: Vec<SocketAddr>,
}

use std::net::{IpAddr, Ipv4Addr};
impl AnnounceResponse {
    // Consumes response buffer of UDP AnnounceRequest
    pub fn new(v: &Vec<u8>) -> Result<Self> {
        let mut action_bytes = &v[0..=3];
        let mut transaction_id_bytes = &v[4..=7];
        let mut interval_bytes = &v[8..=11];
        let mut leechers_bytes = &v[12..=15];
        let mut seeder_bytes = &v[16..=19];
        let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes)?;
        let transaction_id = ReadBytesExt::read_i32::<BigEndian>(&mut transaction_id_bytes)?;
        let interval = ReadBytesExt::read_i32::<BigEndian>(&mut interval_bytes)?;
        let leechers = ReadBytesExt::read_i32::<BigEndian>(&mut leechers_bytes)?;
        let seeders = ReadBytesExt::read_i32::<BigEndian>(&mut seeder_bytes)?;

        // Range where all the IP addresses and Ports are situated
        let x = 20..v.len();

        if action == 3 || (x.len() % 6) != 0 {
            return Err("Server returned error".into());
        }

        let mut peersAddresses = vec![];
        for i in x.step_by(6) {
            let port_bytes = vec![v[i + 4], v[i + 5]];
            let mut port_bytes = &port_bytes[..];
            let port = ReadBytesExt::read_i16::<BigEndian>(&mut port_bytes)?;
            let socket_adr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(v[i], v[i + 1], v[i + 2], v[i + 3])), port as u16);
            peersAddresses.push(socket_adr);
        }

        Ok(AnnounceResponse {
            action,
            transaction_id,
            interval,
            leechers,
            seeders,
            peersAddresses,
        })
    }
}

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

///Type of protocol used to connect to the tracker
#[derive(PartialEq, Debug, Clone)]
pub enum TrackerProtocol {
    UDP,
    HTTP,
}

/// Holds information about the tracker
#[derive(Debug, Clone)]
pub struct Tracker {
    /// Url of the Tracker
    pub url: Url,
    /// Protocol Used by the tracker
    pub protocol: TrackerProtocol,
    /// Socket Address of the remote URL
    pub socket_adr: Option<SocketAddr>,
    /// If the tracker communicated as desired or not
    /// TODO : Remove didItResolve and replace it's usage with Some or None option of socket_adr
    /// address, coz it's basically wheter the socket_adr address option is Some or None
    pub didItResolve: bool,
    /// Data to make connect request
    pub connect_request: Option<ConnectRequest>,
    /// Data received from connect request as response
    pub connect_response: Option<ConnectResponse>,
    /// Data to make announce request
    pub announce_request: Option<AnnounceRequest>,
    /// Data received from announce request as response
    pub announce_response: Option<AnnounceResponse>,
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
