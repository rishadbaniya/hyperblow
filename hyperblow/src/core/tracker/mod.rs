// TODO: Implement DHT

mod announce_req_res;
mod connect_req_res;
mod error_res;

//use announceReqRes::{AnnounceRequest, AnnounceResponse};
//use connectReqRes::{ConnectRequest, ConnectResponse};
use crate::ArcMutex;
use byteorder::{BigEndian, ReadBytesExt};
use connect_req_res::ConnectRequest;
use reqwest::Url;
use std::cmp::PartialEq;
use std::{cell::RefCell, io, net::SocketAddr, rc::Rc, sync::Arc, time::Duration};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio::time::timeout;

use self::connect_req_res::ConnectResponse;

///Type of protocol used to connect to the tracker
#[derive(PartialEq, Debug, Clone)]
pub enum TrackerProtocol {
    UDP,
    TCP,
}

//pub enum TrackerState {
//    waitingForConRes,
//    waitingForAnnRes,
//}
//

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

    pub udp_channel: Option<(UnboundedSender<Vec<u8>>, Arc<Mutex<UnboundedReceiver<Vec<u8>>>>)>,

    /// Data to make connect request
    pub connect_request: Arc<Mutex<TrackerRequest>>,

    /// Data received from connect request as response
    pub connect_response: Arc<Mutex<TrackerResponse>>,

    /// Data to make announce request
    pub announce_request: Arc<Mutex<TrackerRequest>>,

    /// Data received from announce request as response
    pub announce_response: Arc<Mutex<TrackerResponse>>,

    /// Data to make scrape request
    pub scrape_request: Arc<Mutex<TrackerRequest>>,

    /// Data received from scrape request as response
    pub scrape_response: Arc<Mutex<TrackerResponse>>,
}

impl Tracker {
    /// Tries to create a Tracker instance by parsing the given url
    pub fn new(address: &String) -> Result<Tracker, Box<dyn std::error::Error>> {
        let address = Url::parse(address)?;

        let connect_request = ArcMutex!(TrackerRequest::None);
        let connect_response = ArcMutex!(TrackerResponse::None);

        let announce_request = ArcMutex!(TrackerRequest::None);
        let announce_response = ArcMutex!(TrackerResponse::None);

        let scrape_request = ArcMutex!(TrackerRequest::None);
        let scrape_response = ArcMutex!(TrackerResponse::None);

        // TODO: Find out the protocol, if its TCP or UDP for the tracker
        let protocol = TrackerProtocol::UDP;
        let (sd, rv) = mpsc::unbounded_channel::<Vec<u8>>();

        let udp_channel = match protocol {
            TrackerProtocol::UDP => Some((sd, ArcMutex!(rv))),
            TrackerProtocol::TCP => None,
        };

        Ok(Tracker {
            address,
            socketAddrs: None,
            protocol,
            udp_channel,
            connect_request,
            connect_response,
            announce_request,
            announce_response,
            scrape_request,
            scrape_response,
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

    /// Starts running the tracker
    /// socket => Socket through which the tracker will send UDP request and receive UDP response
    pub async fn run(&self, socket: Arc<UdpSocket>) {
        let timeout_duration = |n: u64| Duration::from_secs(15 + 2 ^ n);
        let mut no_of_times_connect_request_timeout = 0;
        //loop {
        let sendConReq = self.sendConnectRequest(socket.clone());
        println!("CAME TO SEND REQUEST");
        match timeout(timeout_duration(no_of_times_connect_request_timeout), sendConReq).await {
            Ok(v) => match v {
                Ok(_) => match self.getResponse().await {
                    Some(res) => {
                        println!("{:?}", res);
                    }
                    None => {}
                },
                Err(e) => {
                    // Error while sending ConnectRequest, probably some kind of socket issue
                }
            },
            Err(_) => {
                // Connect Request timeout error
                if no_of_times_connect_request_timeout != 8 {
                    no_of_times_connect_request_timeout += 1;
                }
            }
        }
        //}
    }

    /// Creates a ConnectRequest instance and tries to send it through the given UDP Socket to the
    /// given UDP Socket Address
    ///
    /// If ConnectRequest is sent, then instance of ConnectRequest is stored in [Tracker] "connect_req" field
    ///
    /// Error :
    /// The only error is IO error passed by tokio::net::UDPSocket
    ///
    /// Panic :
    /// Panic might occur if there is no any Vector inside of self.socketAddrs  
    /// Even if there is a vector, if there is no item at index 0, then a panic will occur
    ///
    pub async fn sendConnectRequest(&self, socket: Arc<UdpSocket>) -> Result<(), io::Error> {
        let connect_req = ConnectRequest::new();
        let connect_req_bytes = connect_req.serializeToBytes();
        // Assumption : Only those Trackers come upto this point whose DNS is resolved from the given URL and contains atleast one resolved socket address
        // that's why unwrap() is called without any worries;
        //
        // TODO : Let's say it contains 10 socket addresses in self.socketAddrs, how to to decide
        // which one to use
        //
        // NOTE : One can use the exact concept that a browser makes use of in order to decide the
        // socket address to be used right now and in the future, as a round robin or ?. Currenlty
        // we are deciding to use the socket at the index 0
        let socketAddrs = self.socketAddrs.as_ref().unwrap().get(0).unwrap();

        return match socket.send_to(connect_req_bytes.as_ref(), socketAddrs).await {
            Ok(_) => {
                {
                    let mut con_req = self.connect_request.lock().await;
                    *con_req = TrackerRequest::ConnectRequest(connect_req);
                }
                Ok(())
            }
            Err(e) => Err(e),
        };
    }

    //pub async fn getConnectResponse() -> Option<ConnectRequest> {}

    /// It will return "None", when the channel is closed
    pub async fn getResponse(&self) -> Option<TrackerResponse> {
        let (_, rx) = self.udp_channel.as_ref().unwrap();
        let mut rx = rx.lock().await;

        return match rx.recv().await {
            Some(d) => {
                println!("{:?}", d);
                // Check for ConnectResponse
                if self.isConnectResponse(&d).await {
                    return if let Ok(cr) = ConnectResponse::from(&d) {
                        Some(TrackerResponse::ConnectResponse(cr))
                    } else {
                        Some(TrackerResponse::None)
                    };
                } else if self.isAnnounceResponse(&d).await {
                    Some(TrackerResponse::None)
                } else {
                    Some(TrackerResponse::None)
                }
            }
            None => None,
        };
    }

    /// Checks from the given buffer, if the given response is a ConnectResponse or not
    pub async fn isConnectResponse(&self, d: &[u8]) -> bool {
        // Check whether the packet is atleast 16 bytes
        if d.len() >= 16 {
            let mut action_bytes = &d[0..=3];
            if let Ok(action) = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes) {
                // Check whether the action is Connect i.e 0
                if action == 0 {
                    let mut transaction_id_bytes = &d[4..=7];
                    if let Ok(transaction_id) = ReadBytesExt::read_i32::<BigEndian>(&mut transaction_id_bytes) {
                        let connect_req_transaction_id = {
                            let connect_request = self.connect_request.lock().await;
                            if let TrackerRequest::ConnectRequest(ref connect_request) = *connect_request {
                                connect_request.transaction_id
                            } else {
                                // Make sure that you store a ConnectRequest in the place of
                                // "self.connect_request"
                                // A random number just to make the function false in the next
                                // step i.e comparing the connect req
                                0
                            }
                        };
                        // Check whether the transaction_id matches with the ConnectRequest or not
                        return connect_req_transaction_id == transaction_id;
                    } else {
                        return false;
                    }
                }
            } else {
                return false;
            }
            return true;
        }
        return false;
    }

    /// Checks from the given buffer, if it's a ConnectResponse or not
    pub async fn isAnnounceResponse(&self, d: &[u8]) -> bool {
        false // Dummy
    }

    pub async fn isScrapeResponse(&self, d: &[u8]) -> bool {
        false // Dummy
    }

    /// Checks from the given buffer, if the given response is an Error or not
    pub async fn isErrorResponse(&self, d: &[u8]) -> bool {
        //let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes);
        false //dummy
    }
}

#[derive(Debug)]
pub enum TrackerResponse {
    ConnectResponse(ConnectResponse),
    Error,
    None,
}

#[derive(Debug)]
pub enum TrackerRequest {
    ConnectRequest(ConnectRequest),
    //AnnounceRequest(AnnounceRequest),
    None,
}
