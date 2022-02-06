// This module handles everything related with a Tracker :
// The UDP Tracker Protocol is followed from : http://www.bittorrent.org/beps/bep_0015.html
// TODO : {
//    1 : Add Scrape Request
//    2 : Add TCP Tracker request
// }

use crate::Result;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use reqwest::Url;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;

const TRACKER_ERROR: &str =
    "There is something wrong with the torrent file you provided \n Couldn't parse one of the tracker URL";
//
// Struct to handle the message to be sent to "Connect" on the UDP Tracker
// Used to create a "16 byte" buffer to make "Connect Request"
//
// Connect Request Bytes Structure:
//
// Offset  Size            Name            Value
// 0       64-bit integer  protocol_id     0x41727101980 // magic constant
// 8       32-bit integer  action          0 // connect
// 12      32-bit integer  transaction_id
// 16
//
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

// Struct to handle response message from "Connect" request to the UDP Tracker
// Used to create an instance of AnnounceRequest
// Connect Response Bytes Structure from the UDP Tracker Protocol :
//
// Offset  Size            Name            Value
// 0       32-bit integer  action          0 // connect
// 4       32-bit integer  transaction_id
// 8       64-bit integer  connection_id
// 16
#[derive(Debug, Clone)]
pub struct ConnectResponse {
    pub action: i32,
    pub transaction_id: i32,
    pub connection_id: i64,
}

impl ConnectResponse {
    pub fn from_array_buffer(v: Vec<u8>) -> Self {
        let mut action_bytes = &v[0..=3];
        let mut transaction_id_bytes = &v[4..=7];
        let mut connection_id_bytes = &v[8..=15];
        let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes).unwrap();
        let transaction_id =
            ReadBytesExt::read_i32::<BigEndian>(&mut transaction_id_bytes).unwrap();
        let connection_id = ReadBytesExt::read_i64::<BigEndian>(&mut connection_id_bytes).unwrap();
        Self {
            action,
            transaction_id,
            connection_id,
        }
    }
}

// Struct to handle "Announce" request message
// Used to create a "98 byte" buffer to make "Announce Request"
// Reference : http://www.bittorrent.org/beps/bep_0015.html
//
// IPv4 announce request Bytes Structure:
// Offset  Size    Name    Value
// 0       64-bit integer  connection_id   The connection id acquired from establishing the connection.
// 8       32-bit integer  action          Action. in this case, 1 for announce. See : https://www.rasterbar.com/products/libtorrent/udp_tracker_protocol.html#actions
// 12      32-bit integer  transaction_id  Randomized by client
// 16      20-byte string  info_hash       The info-hash of the torrent you want announce yourself in.
// 36      20-byte string  peer_id         Your peer id. (Peer ID Convention : https://www.bittorrent.org/beps/bep_0020.html)
// 56      64-bit integer  downloaded      The number of byte you've downloaded in this session.
// 64      64-bit integer  left            The number of bytes you have left to download until you're finished.
// 72      64-bit integer  uploaded        The number of bytes you have uploaded in this session.
// 80      32-bit integer  event           0 // 0: none; 1: completed; 2: started; 3: stopped
// 84      32-bit integer  IP address      Your ip address. Set to 0 if you want the tracker to use the sender of this UDP packet.u
// 88      32-bit integer  key             A unique key that is randomized by the client.
// 92      32-bit integer  num_want        The maximum number of peers you want in the reply. Use -1 for default.
// 96      16-bit integer  port            The port you're listening on.
// 98
#[derive(Debug, Clone)]
pub struct AnnounceRequest {
    connection_id: Option<i64>,
    action: i32,
    transaction_id: Option<i32>,
    info_hash: Option<[u8; 20]>,
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
        let peer_id_slice = b"-BOWxxx-yyyyyyyyyyyy";
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
        bytes.put_slice(&self.info_hash.unwrap());
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

    pub fn set_info_hash(&mut self, v: [u8; 20]) {
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
    pub fn new(v: &Vec<u8>) -> Self {
        let mut action_bytes = &v[0..=3];
        let mut transaction_id_bytes = &v[4..=7];
        let mut interval_bytes = &v[8..=11];
        let mut leechers_bytes = &v[12..=15];
        let mut seeder_bytes = &v[16..=19];
        let mut port_bytes = &v[24..=25];
        let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes).unwrap();
        let transaction_id =
            ReadBytesExt::read_i32::<BigEndian>(&mut transaction_id_bytes).unwrap();
        let interval = ReadBytesExt::read_i32::<BigEndian>(&mut interval_bytes).unwrap();
        let leechers = ReadBytesExt::read_i32::<BigEndian>(&mut leechers_bytes).unwrap();
        let seeders = ReadBytesExt::read_i32::<BigEndian>(&mut seeder_bytes).unwrap();
        let port = ReadBytesExt::read_i16::<BigEndian>(&mut port_bytes).unwrap();
        let socket_adr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(v[20], v[21], v[22], v[23])),
            port as u16,
        );
        let x = 20..v.len();
        let peersAddresses = vec![socket_adr];

        AnnounceResponse {
            action,
            transaction_id,
            interval,
            leechers,
            seeders,
            peersAddresses,
        }
    }
}
// Offset          Size            Name            Value
// 0               64-bit integer  connection_id
// 8               32-bit integer  action          2 // scrape
// 12              32-bit integer  transaction_id
// 16 + 20 * n     20-byte string  info_hash
// 16 + 20 * N
//
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
    pub url: Url,                                    // Url of the Tracker
    pub protocol: TrackerProtocol,                   // Protocol Used by the tracker
    pub socket_adr: Option<SocketAddr>,              // Socket Address of the remote URL
    pub didItResolve: bool, // If the tracker communicated as desired or not
    pub connect_request: Option<ConnectRequest>, // Data to make connect request
    pub connect_response: Option<ConnectResponse>, // Data received from connect request as response
    pub announce_request: Option<AnnounceRequest>, // Data to make announce request
    pub announce_response: Option<AnnounceResponse>, // Data received from announce request as response
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

    /// Create list of "Tracker" from data in the
    /// announce and announce_list field of "FileMeta"
    pub fn getTrackers(
        announce: &String,
        announce_list: &Vec<Vec<String>>,
    ) -> Vec<Arc<Mutex<RefCell<Tracker>>>> {
        let mut trackers: Vec<_> = Vec::new();

        trackers.push(Arc::new(Mutex::new(RefCell::new(Tracker::new(announce)))));

        for tracker_url in announce_list {
            trackers.push(Arc::new(Mutex::new(RefCell::new(Tracker::new(
                &tracker_url[0],
            )))));
        }
        trackers
    }
}

/// To be called at the first step of communicating with the UDP Tracker Server
pub async fn connect_request(
    transaction_id: i32,
    socket: &UdpSocket,
    to: &SocketAddr,
    tracker: Arc<Mutex<RefCell<Tracker>>>,
) -> Result<()> {
    let tracker_lock = tracker.lock().unwrap();
    let mut tracker_borrow_mut = tracker_lock.borrow_mut();
    let mut connect_request = ConnectRequest::empty();
    connect_request.set_transaction_id(transaction_id);
    tracker_borrow_mut.connect_request = Some(connect_request.clone());
    let buf = connect_request.getBytesMut();
    socket.send_to(&buf, to).await?;
    tracker_borrow_mut.connect_request = Some(connect_request);
    Ok(())
}

/// To be called after having an instance of "ConnectResponse" which can be obtained
/// after making a call to "connect_request"
pub async fn annnounce_request(
    connection_response: ConnectResponse,
    socket: &UdpSocket,
    to: &SocketAddr,
    info_hash: Vec<u8>,
    tracker: Arc<Mutex<RefCell<Tracker>>>,
) -> Result<()> {
    // Note : The message sent from announce_request is kinda dynamic in a sense that
    // it has unknown amount of peers ip addresses and ports
    // Buffer to store the response
    let tracker_lock = tracker.lock().unwrap();
    let mut tracker_borrow_mut = tracker_lock.borrow_mut();

    let mut announce_request = AnnounceRequest::empty();
    announce_request.set_connection_id(connection_response.connection_id);
    announce_request.set_transaction_id(connection_response.transaction_id);
    announce_request.set_info_hash(info_hash.try_into().unwrap());
    announce_request.set_downloaded(0);
    announce_request.set_uploaded(0);
    announce_request.set_uploaded(0);
    announce_request.set_left(100);
    announce_request.set_port(8001);
    announce_request.set_key(20);
    socket.send_to(&announce_request.getBytesMut(), to).await?;
    tracker_borrow_mut.announce_request = Some(announce_request);
    //let (_, _) = timeout(Duration::from_secs(4), socket.recv_from(&mut response)).await??;
    //let announce_response = AnnounceResponse::new(&response);
    Ok(())
}
