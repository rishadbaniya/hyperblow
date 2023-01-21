// TODO: Implement DHT

mod announce_req_res;
mod connect_req_res;

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
    pub connect_request: Rc<RefCell<TrackerRequest>>,

    /// Data received from connect request as response
    pub connect_response: Rc<RefCell<TrackerResponse>>,

    /// Data to make announce request
    pub announce_request: Rc<RefCell<TrackerRequest>>,

    /// Data received from announce request as response
    pub announce_response: Rc<RefCell<TrackerResponse>>,

    /// Data to make scrape request
    pub scrape_request: Rc<RefCell<TrackerRequest>>,

    /// Data received from scrape request as response
    pub scrape_response: Rc<RefCell<TrackerResponse>>,
}

impl Tracker {
    /// Tries to create a Tracker instance by parsing the given url
    pub fn new(address: &String) -> Result<Tracker, Box<dyn std::error::Error>> {
        let address = Url::parse(address)?;

        let connect_request = Rc::new(RefCell::new(TrackerRequest::None));
        let connect_response = Rc::new(RefCell::new(TrackerResponse::None));

        let announce_request = Rc::new(RefCell::new(TrackerRequest::None));
        let announce_response = Rc::new(RefCell::new(TrackerResponse::None));

        let scrape_request = Rc::new(RefCell::new(TrackerRequest::None));
        let scrape_response = Rc::new(RefCell::new(TrackerResponse::None));

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

        loop {
            let mut no_of_times_connect_request_timeout = 0;
            let sendConReq = self.sendConnectRequest(socket.clone());

            match timeout(timeout_duration(no_of_times_connect_request_timeout), sendConReq).await {
                Ok(v) => match v {
                    Ok(_) => {}
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
        }
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
                    let mut con_req = self.connect_request.borrow_mut();
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

        match rx.recv().await {
            Some(x) => {
                // Check for ConnectResponse
            }
            None => {}
        };
        None
    }

    /// Checks from the given buffer, if it's a ConnectResponse or not
    pub fn isConnectResponse(&self, d: &Vec<u8>) -> bool {
        // Check whether the packet is atleast 16 bytes
        if d.len() >= 16 {
            let mut action_bytes = &d[0..=3];
            if let Ok(action) = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes) {
                // Check whether the action is Connect i.e 0
                if action == 0 {
                    let mut transaction_id_bytes = &d[4..=7];
                    if let Ok(transaction_id) = ReadBytesExt::read_i32::<BigEndian>(&mut transaction_id_bytes) {
                        let connect_req_transaction_id = {
                            let connect_request = self.connect_request.borrow();
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
}

#[derive(Debug)]
enum TrackerResponse {
    None,
}

#[derive(Debug)]
enum TrackerRequest {
    ConnectRequest(ConnectRequest),
    //AnnounceRequest(AnnounceRequest),
    None,
}
