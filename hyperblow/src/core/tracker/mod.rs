// TODO: Implement DHT

mod announce_req_res;

//use announceReqRes::{AnnounceRequest, AnnounceResponse};
//use connectReqRes::{ConnectRequest, ConnectResponse};
use reqwest::Url;
use std::cmp::{Eq, PartialEq};
use std::net::SocketAddr;
use tokio::sync::oneshot;
use tokio::sync::oneshot::{Receiver, Sender};

///Type of protocol used to connect to the tracker
#[derive(PartialEq, Debug, Clone)]
pub enum TrackerProtocol {
    UDP,
    HTTP,
}

/// A tracker in BitTorrent is simply, a "URL", that uses certian request and response technique in
/// order to get information about peers
///
///One way to avoid using Trackers is using DHT(Distributed Hash Table).
#[derive(Debug)]
pub struct Tracker {
    /// A Url instance of reqwest crate, because it supports DNS resolving as well as distinctly parses the given string of URL
    pub address: Url,

    /// Protocol Used by the tracker
    pub protocol: TrackerProtocol,

    /// A single domain can have multiple A and AAAA records in its Resource Records, which means multiple socket addresses for a same domain i.e it can resolve to multiple ip address, may it be ipv4 or ipv6
    ///
    /// A "None" in the socketAddrs means that the DNS wasn't resolved for the given domain of
    /// Tracker
    pub socketAddrs: Option<Vec<SocketAddr>>,
    //    /// Data to make connect request
    //    pub connect_request: Option<ConnectRequest>,
    //
    //    /// Data received from connect request as response
    //    pub connect_response: Option<ConnectResponse>,
    //
    //    /// Data to make announce request
    //    pub announce_request: Option<AnnounceRequest>,
    //
    //    /// Data received from announce request as response
    //    pub announce_response: Option<AnnounceResponse>,
    pub udp_channel: Option<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>,
}

impl Tracker {
    /// Tries to create a Tracker instance by parsing the given url
    pub fn new(address: &String) -> Result<Tracker, Box<dyn std::error::Error>> {
        let address = Url::parse(address)?;
        let udp_channel = Some(oneshot::channel::<Vec<u8>>());

        // TODO: Find out the protocol, if its TCP or UDP for the tracker
        let protocol = TrackerProtocol::UDP;

        Ok(Tracker {
            address,
            socketAddrs: None,
            protocol,
            udp_channel,
            //           connect_request: None,
            //           connect_response: None,
            //           announce_request: None,
            //           announce_response: None,
        })
    }

    /// Tries to resolve the A and AAAA records of the domain
    pub fn resolveSocketAddr(&mut self) -> bool {
        if let Ok(addrs) = self.address.socket_addrs(|| None) {
            self.socketAddrs = Some(addrs);
            true
        } else {
            false
        }
    }

    // Compares given socket address to the trackers list of socket addresses,
    // if it matches any one of it, then we can say that the given socket address belongs
    // to the tracker i.e the socket address is equal to the Tracker
    pub fn isEqualTo(&self, sAdr1: &SocketAddr) -> bool {
        if let Some(ref sAdresses) = self.socketAddrs {
            for sAdr2 in sAdresses {
                return *sAdr1 == *sAdr2;
            }
        }
        false
    }
}
