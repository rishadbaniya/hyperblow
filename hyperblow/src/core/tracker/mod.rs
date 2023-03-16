// TODO: Implement DHT

mod announce_req_res;
mod connect_req_res;
mod error_res;

use crate::core::peer::Peer;
use crate::core::state::State;
use crate::ArcMutex;
use byteorder::{BigEndian, ReadBytesExt};
use rand::{thread_rng, Rng};
use reqwest::Url;
use std::cmp::PartialEq;
use std::time::Instant;
use std::{io, net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

use self::announce_req_res::{AnnounceRequest, AnnounceResponse};
use self::connect_req_res::{ConnectRequest, ConnectResponse};

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
    /// The state of the torrent file
    pub torrent_state: Arc<State>,

    /// A Url instance of reqwest crate, because it supports DNS resolving as well as distinctly parses the given string of URL
    pub address: Url,

    /// Protocol Used by the tracker
    pub protocol: TrackerProtocol,

    /// A single domain can have multiple A and AAAA records in its Resource Records, which means multiple socket addresses for a same domain i.e it can resolve to multiple ip address, may it be ipv4 or ipv6
    ///
    /// A "None" in the socketAddrs means that the DNS wasn't resolved for the given domain of
    /// Tracker
    pub socketAddrs: Option<Vec<SocketAddr>>,

    /// A sender part of a channel, to send the [Peer] instance to the runDownload()
    /// method of [TorrentFile], created by collecting the socket address of Peers
    /// from the Trackers, received while getting AnnounceResponse or ScrapeResponse
    /// from other trackers.
    pub peer_sender: Arc<UnboundedSender<Peer>>,

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
    // TODO : Store UDP Socket here in the struct
}

impl Tracker {
    /// Tries to create a Tracker instance by parsing the given url
    pub fn new(address: &String, torrent_state: Arc<State>, peer_sender: Arc<UnboundedSender<Peer>>) -> Result<Tracker, Box<dyn std::error::Error>> {
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
            torrent_state,
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
            peer_sender,
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
        // A timeout duration for all types of responses
        let timeout_duration = |n: u64| Duration::from_secs(15 + 2 ^ n);

        let mut no_of_times_connect_request_timeout = 0;

        loop {
            let sendConReq = self.sendConnectRequest(socket.clone());
            match timeout(timeout_duration(no_of_times_connect_request_timeout), sendConReq).await {
                Ok(v) => match v {
                    // TODO : Replace with get_connect_response();
                    Ok(_) => match self.getResponse().await {
                        Some(res) => {
                            // Save the received ConnectResponse in self.connect_response
                            {
                                let mut connect_response = self.connect_response.lock().await;
                                *connect_response = res;
                            }

                            // According to BEP-15, client can use a Connection ID until one minute after it has received it.
                            //
                            // This means we can't use the connection_id stored inside of ConnectResponse after 1 minute
                            let connection_id_timeout_duration = Duration::from_secs(60);
                            let duration_since_connect_response = Instant::now();

                            'announce: loop {
                                let now = Instant::now();
                                let now = now.duration_since(duration_since_connect_response);

                                let mut no_of_times_announce_request_timeout = 0;
                                if now <= connection_id_timeout_duration {
                                    match timeout(
                                        timeout_duration(no_of_times_announce_request_timeout),
                                        self.send_announce_request(socket.clone()),
                                    )
                                    .await
                                    {
                                        Ok(vv) => {
                                            match vv {
                                                // TODO : Replace with get_announce_response();
                                                Ok(_) => match self.getResponse().await {
                                                    Some(res) => {
                                                        if let TrackerResponse::AnnounceResponse(ref ar) = res {
                                                            //println!("The interval is {}", ar.interval);
                                                            let sleep_duration = Duration::from_secs(ar.interval as u64);
                                                            {
                                                                for peer_socket_adr in ar.peersAddresses.clone() {
                                                                    let peer = Peer::new(peer_socket_adr, self.torrent_state.clone());
                                                                    self.peer_sender.send(peer);
                                                                }
                                                                let mut announce_response = self.announce_response.lock().await;
                                                                *announce_response = res;
                                                            }
                                                            sleep(sleep_duration).await;
                                                            break 'announce;
                                                        }

                                                        // Save the received AnnounceResponse in self.announannounce_response
                                                    }
                                                    None => {
                                                        //println!("GOT ERROR");
                                                    }
                                                },
                                                Err(e) => {
                                                    // Error while sending AnnounceRequest, probably some kind of socket issue
                                                    //println!("CONNECT REQUEST SOCKET ISSUE {:?}", e.to_string());
                                                    sleep(Duration::from_secs(1000)).await;
                                                    // TODO : Replace this with some actual solution rather than sleeping
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            if no_of_times_announce_request_timeout <= 8 {
                                                no_of_times_announce_request_timeout += 1;
                                            }
                                            // Announce Request timeout error
                                        }
                                    }
                                }
                            }
                        }
                        None => {}
                    },
                    Err(e) => {
                        // Error while sending ConnectRequest, probably some kind of socket issue
                        //println!("CONNECT REQUEST SOCKET ISSUE {:?}", e.to_string());
                        sleep(Duration::from_secs(1000)).await;

                        // TODO : Replace this with some actual solution rather than sleeping
                    }
                },
                Err(_) => {
                    //println!("GOT TIMEOUT ERROR FOR CONNECT RESPONSE");
                    // Connect Request timeout error
                    if no_of_times_connect_request_timeout <= 8 {
                        no_of_times_connect_request_timeout += 1;
                    }
                }
            }
            //println!("ENDED PRETTY QUICK");
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
                let mut con_req = self.connect_request.lock().await;
                *con_req = TrackerRequest::ConnectRequest(connect_req);
                Ok(())
            }
            Err(e) => Err(e),
        };
    }

    pub async fn send_announce_request(&self, socket: Arc<UdpSocket>) -> Result<(), io::Error> {
        let mut announce_req = AnnounceRequest::new();
        {
            let connect_response = self.connect_response.lock().await;
            if let TrackerResponse::ConnectResponse(ref c_res) = *connect_response {
                announce_req.set_connection_id(c_res.connection_id);
                announce_req.set_transaction_id(c_res.transaction_id);
                announce_req.set_info_hash(&self.torrent_state.info_hash);
                announce_req.set_downloaded(1000); // TODO : Replace with actual downloaded bytes
                announce_req.set_uploaded(1000); // TODO : Replace with actual uploaded bytes
                announce_req.set_left(5000); // TODO : Replace with actual left bytes
                {
                    let ports = self.torrent_state.udp_ports.lock().await;
                    if let Some(port) = ports.get(0) {
                        announce_req.set_port(*port as i16);
                    }
                }
                let mut rng = thread_rng();
                announce_req.set_key(rng.gen());
            }
        }

        if let Some(announce_req_bytes) = announce_req.serialize_to_bytes() {
            let socketAddrs = self.socketAddrs.as_ref().unwrap().get(0).unwrap();
            return match socket.send_to(&announce_req_bytes, socketAddrs).await {
                Ok(_) => {
                    let mut ann_req = self.announce_request.lock().await;
                    *ann_req = TrackerRequest::AnnounceRequest(announce_req);
                    Ok(())
                }
                Err(e) => Err(e),
            };
        } else {
            //println!("Error in serializing to bytes");
            // TODO : Write else case
        }

        Ok(())
    }

    /// It pushes the peers achieved from Announce into the "peers" field of
    /// the "state" field of [Tracker]
    ///
    /// It checks, if the Peer already exists in the list, if it does, then it simply
    /// pushes a pointer of the [Tracker] into the "info" field of the "tracker" field
    /// of the [Peer]
    ///
    /// socket_adr : Socket Address of the peer that is to be pushed
    async fn push_to_peers(&self, peer_socket_adr: SocketAddr) {
        let mut peers = self.torrent_state.peers.lock().await;
        let mut DOES_PEER_ALREADY_EXIST = false;

        for peer in &(*peers) {
            DOES_PEER_ALREADY_EXIST = (peer.socket_adr == peer_socket_adr);
        }

        if !DOES_PEER_ALREADY_EXIST {
            peers.push(Peer::new(peer_socket_adr, self.torrent_state.clone()));
        }
    }

    /// It will return "None", when the channel is closed
    pub async fn getResponse(&self) -> Option<TrackerResponse> {
        let NONE = Some(TrackerResponse::None);
        let (_, rx) = self.udp_channel.as_ref().unwrap();
        let mut rx = rx.lock().await;

        return match rx.recv().await {
            Some(d) => {
                // Check for ConnectResponse
                if self.isConnectResponse(&d).await {
                    //println!("GOT A CONNECT RESPONSE HERE");
                    //println!("{:?}", self.torrents);

                    return if let Ok(cr) = ConnectResponse::from(&d) {
                        Some(TrackerResponse::ConnectResponse(cr))
                    } else {
                        NONE
                    };
                } else if self.isAnnounceResponse(&d).await {
                    //println!("GOT ANNOUNCE RESPONSE");

                    return if let Ok(ar) = AnnounceResponse::from(&d) {
                        Some(TrackerResponse::AnnounceResponse(ar))
                    } else {
                        NONE
                    };
                } else {
                    NONE
                }
            }
            None => None,
        };
    }

    /// Checks from the given buffer, if the given response is a ConnectResponse or not
    pub async fn isConnectResponse(&self, d: &[u8]) -> bool {
        let mut IS_CONNECT_RESPONSE = false;

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
                        // Check whether the transaction_id in ConnectRequest matches with the ConnectResponse or not
                        IS_CONNECT_RESPONSE = connect_req_transaction_id == transaction_id;
                    }
                }
            }
        }
        return IS_CONNECT_RESPONSE;
    }

    /// Checks from the given buffer, if it's a ConnectResponse or not
    pub async fn isAnnounceResponse(&self, d: &[u8]) -> bool {
        let mut IS_ANNOUNCE_RESPONSE = false;
        // Check whether the packet is atleast 16 bytes
        if d.len() >= 20 {
            let mut action_bytes = &d[0..=3];
            if let Ok(action) = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes) {
                // Check whether the action is Announce i.e 1
                if action == 1 {
                    let mut transaction_id_bytes = &d[4..=7];
                    if let Ok(transaction_id) = ReadBytesExt::read_i32::<BigEndian>(&mut transaction_id_bytes) {
                        let announce_req_transaction_id = {
                            let announce_request = self.announce_request.lock().await;
                            if let TrackerRequest::AnnounceRequest(ref announce_request) = *announce_request {
                                announce_request.transaction_id.unwrap()
                            } else {
                                // Make sure that you store a AnnounceRequest in the place of "self.announce_request"
                                //
                                // A random number just to make the function false in the next
                                // step i.e comparing the connect re
                                0
                            }
                        };
                        IS_ANNOUNCE_RESPONSE = announce_req_transaction_id == transaction_id;
                    }
                }
            }
        }

        return IS_ANNOUNCE_RESPONSE;
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
    AnnounceResponse(AnnounceResponse),
    Error,
    None,
}

#[derive(Debug)]
pub enum TrackerRequest {
    ConnectRequest(ConnectRequest),
    AnnounceRequest(AnnounceRequest),
    None,
}
