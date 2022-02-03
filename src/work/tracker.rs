// This module handles everything required to do with a Tracker
// TODO : Write a small note about Trackers
//

use super::torrent_parser::FileMeta;
use reqwest::Url;

const TRACKER_ERROR: &str =
    "There is something wrong with the torrent file you provided \n Couldn't parse a tracker URL";

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
