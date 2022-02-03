// This module handles everything required to do with a Tracker
// The protocol is followed from : http://www.bittorrent.org/beps/bep_0015.html

use super::torrent_parser::FileMeta;
use reqwest::Url;

const TRACKER_ERROR: &str =
    "There is something wrong with the torrent file you provided \n Couldn't parse a tracker URL";

// Struct to hold data for "Announce" of http://www.bittorrent.org/beps/bep_0015.html
// and create a "98 byte" buffer to store the data
pub struct Announce {
    connection_id: i64,
    action: i32,
    transaction_id: i32,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    downloaded: i64,
    left: i64,
    uploaded: i64,
    event: i32,
    ip_address: i32,
    key: i32,
    num_want: i32,
    port: i16,
}

//Type of protocol used to connect to the tracker
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
