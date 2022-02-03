// This module handles everything required to do with a Tracker
// The protocol is followed from : http://www.bittorrent.org/beps/bep_0015.html

use bytes::{BufMut, BytesMut};
use reqwest::Url;

const TRACKER_ERROR: &str =
    "There is something wrong with the torrent file you provided \n Couldn't parse a tracker URL";

// Struct to hold data for "Announce"
// and create a "98 byte" buffer to make "Announce Request"
// Reference : http://www.bittorrent.org/beps/bep_0015.html
// IPv4 announce requst:
// Offset  Size    Name    Value
// 0       64-bit integer  connection_id
// 8       32-bit integer  action          1 // announce
// 12      32-bit integer  transaction_id
// 16      20-byte string  info_hash
// 36      20-byte string  peer_id
// 56      64-bit integer  downloaded
// 64      64-bit integer  left
// 72      64-bit integer  uploaded
// 80      32-bit integer  event           0 // 0: none; 1: completed; 2: started; 3: stopped
// 84      32-bit integer  IP address      0 // default
// 88      32-bit integer  key
// 92      32-bit integer  num_want        -1 // default
// 96      16-bit integer  port
// 98
pub struct Announce {
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
impl Announce {
    // Creates an empty Announce instance
    pub fn empty() -> Self {
        Announce {
            connection_id: None,
            action: Some(1),
            transaction_id: None,
            info_hash: None,
            peer_id: None,
            downloaded: None,
            left: None,
            uploaded: None,
            event: None,
            ip_address: None,
            key: None,
            num_want: None,
            port: None,
        }
    }

    // Consumes the Announce and gives you a Buffer of 98 bytes
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

    pub fn set_action(&mut self, v: i32) {
        self.action = Some(v);
    }

    pub fn set_transaction_id(&mut self, v: i32) {
        self.action = Some(v);
    }

    pub fn set_info_hash(&mut self, v: [u8; 20]) {
        self.info_hash = Some(v);
    }

    pub fn set_peer_id(&mut self, v: [u8; 20]) {
        self.peer_id = Some(v);
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
        Tracker { url, protocol }
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
