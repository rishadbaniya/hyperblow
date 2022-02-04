// This module handles everything required to do with a Tracker :
// The UDP Tracker Protocol is followed from : http://www.bittorrent.org/beps/bep_0015.html
use super::torrent_parser::FileMeta;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use reqwest::Url;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::{net::UdpSocket, time::timeout};

const TRACKER_ERROR: &str =
    "There is something wrong with the torrent file you provided \n Couldn't parse one of the tracker URL";

//
// Struct to handle "Connect" request message and
// used to create a "16 byte" buffer to make "Connect Request"
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

// Struct to handle response from "Connect" request to the UDP Tracker
// Used to create a an instance of AnnounceRequest
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
    pub fn from_array_buffer(v: [u8; 20]) -> Self {
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
pub struct AnnounceRequest {
    connection_id: Option<i64>,
    action: Option<i32>,
    transaction_id: Option<i32>,
    info_hash: Option<[u8; 20]>,
    peer_id: Option<[u8; 20]>,
    downloaded: Option<i64>,
    left: Option<i64>,
    uploaded: Option<i64>,
    event: Option<i32>,
    ip_address: Option<i32>,
    key: Option<i32>,
    num_want: Option<i32>,
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
            action: Some(1),
            transaction_id: None,
            info_hash: None,
            peer_id: Some(peer_id),
            downloaded: None,
            left: None,
            uploaded: None,
            event: Some(1),
            ip_address: Some(0),
            key: None,
            num_want: Some(-1),
            port: None,
        }
    }

    // Consumes the Announce instance and gives you a Buffer of 98 bytes that you
    // can use to make Announce Request in UDP
    // TODO : Return a Result<BytesMut> to handle error propagated by ".unwrap()"
    pub fn getBytesMut(&self) -> BytesMut {
        let mut bytes = BytesMut::with_capacity(98);
        bytes.put_i64(self.connection_id.unwrap());
        bytes.put_i32(self.action.unwrap());
        bytes.put_i32(self.transaction_id.unwrap());
        bytes.put_slice(&self.info_hash.unwrap());
        bytes.put_slice(&self.peer_id.unwrap());
        bytes.put_i64(self.downloaded.unwrap());
        bytes.put_i64(self.left.unwrap());
        bytes.put_i64(self.uploaded.unwrap());
        bytes.put_i32(self.event.unwrap());
        bytes.put_i32(self.ip_address.unwrap());
        bytes.put_i32(self.key.unwrap());
        bytes.put_i32(self.num_want.unwrap());
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
#[derive(Debug)]
pub struct AnnounceResponse {
    action: i32,
    transaction_id: i32,
    interval: i32,
    leechers: i32,
    seeders: i32,
    //RemoteSocketAddresses: Vec<SocketAddr>,
}

impl AnnounceResponse {
    // Consumes response buffer of UDP AnnounceRequest
    pub fn new(v: &Vec<u8>) -> Self {
        let mut action_bytes = &v[0..=3];
        let mut transaction_id_bytes = &v[4..=7];
        let mut interval_bytes = &v[8..=15];
        let mut leechers_bytes = &v[8..=15];
        let mut seeder_bytes = &v[8..=15];
        let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes).unwrap();
        let transaction_id =
            ReadBytesExt::read_i32::<BigEndian>(&mut transaction_id_bytes).unwrap();
        let interval = ReadBytesExt::read_i32::<BigEndian>(&mut interval_bytes).unwrap();
        let leechers = ReadBytesExt::read_i32::<BigEndian>(&mut leechers_bytes).unwrap();
        let seeders = ReadBytesExt::read_i32::<BigEndian>(&mut seeder_bytes).unwrap();
        AnnounceResponse {
            action,
            transaction_id,
            interval,
            leechers,
            seeders,
        }
    }
}

//Type of protocol used to connect to the tracker
#[derive(PartialEq)]
pub enum TrackerProtocol {
    UDP,
    HTTP,
}

// Holds information about the tracker
pub struct Tracker {
    pub url: Url,
    pub protocol: TrackerProtocol,
    pub socket_adr: Option<SocketAddr>,
}

impl Tracker {
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
        }
    }

    /// Create list of "Tracker" from data in the
    /// announce and announce_list field of "FileMeta"
    pub fn getTrackers(announce: &String, announce_list: &Vec<Vec<String>>) -> Vec<Tracker> {
        let mut trackers: Vec<_> = Vec::new();

        trackers.push(Tracker::new(announce));

        for trackerUrl in announce_list {
            trackers.push(Tracker::new(&trackerUrl[0]));
        }
        trackers
    }
}

pub fn generateTrackerList(fileMeta: &FileMeta) -> Vec<Tracker> {
    let announce_list = fileMeta.announce_list.as_ref().unwrap();
    let trackers: Vec<Tracker> = Tracker::getTrackers(&fileMeta.announce, announce_list);
    trackers
}

use crate::Result;
pub async fn connect_request(
    transaction_id: i32,
    socket: &UdpSocket,
    to: &SocketAddr,
) -> Result<ConnectResponse> {
    // Creates a buffer to receive response
    let mut response = [0u8; 20];
    let mut connect_request = ConnectRequest::empty();
    connect_request.set_transaction_id(transaction_id);
    let _ = socket.send_to(&connect_request.getBytesMut(), to).await?;
    let _ = timeout(Duration::from_secs(1), socket.recv_from(&mut response)).await?;
    let connect_response = ConnectResponse::from_array_buffer(response);
    Ok(connect_response)
}

pub async fn annnounce_request(
    connection_response: ConnectResponse,
    socket: &UdpSocket,
    to: &SocketAddr,
    info_hash: Vec<u8>,
) -> Result<AnnounceResponse> {
    let mut response = vec![0; 1024];
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
    let _ = socket.send_to(&announce_request.getBytesMut(), to).await?;
    let _ = timeout(Duration::from_secs(3), socket.recv_from(&mut response)).await?;
    let announce_response = AnnounceResponse::new(&response);
    Ok(announce_response)
}
