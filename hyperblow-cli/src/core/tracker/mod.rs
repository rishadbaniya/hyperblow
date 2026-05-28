// TODO: Implement DHT

mod announce_req_res;
mod connect_req_res;
mod error_res;

use self::{
    announce_req_res::{AnnounceRequest, AnnounceResponse},
    connect_req_res::{ConnectRequest, ConnectResponse},
};
use crate::{
    core::{peer::Peer, protocol::PEER_ID, state::State},
    ACell, ArcMutex,
};
use byteorder::{BigEndian, ReadBytesExt};
use crossbeam::atomic::AtomicCell;
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;
use std::{
    fmt::Display,
    fmt::Write,
    io,
    net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::{
    net::UdpSocket,
    sync::{
        mpsc,
        mpsc::{UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    time::{sleep, timeout},
};
use url::Url;

type UdpTrackerChannel = (UnboundedSender<Vec<u8>>, Arc<Mutex<UnboundedReceiver<Vec<u8>>>>);
type TrackerResult<T> = Result<T, TrackerError>;

#[derive(Debug, Error)]
pub enum TrackerError {
    #[error("unsupported tracker protocol: {0}")]
    UnsupportedProtocol(String),

    #[error("invalid tracker URL")]
    InvalidUrl(#[from] url::ParseError),

    #[error("HTTP tracker request failed")]
    HttpRequest(#[from] reqwest::Error),

    #[error("tracker bencode response could not be decoded")]
    Bencode(#[from] serde_bencode::Error),

    #[error("tracker returned failure: {0}")]
    TrackerFailure(String),

    #[error("invalid peer IP address in tracker response: {ip}")]
    InvalidPeerIp { ip: String, source: AddrParseError },

    #[error("compact {family} peer list length must be a multiple of {chunk_size}, got {len}")]
    InvalidCompactPeerList {
        family: &'static str,
        chunk_size: usize,
        len: usize,
    },
}

#[derive(Debug, Deserialize)]
struct HttpAnnounceResponse {
    interval: Option<i64>,
    complete: Option<i64>,
    incomplete: Option<i64>,
    peers: Option<HttpPeers>,
    #[serde(rename = "peers6", with = "serde_bytes", default)]
    peers6: Vec<u8>,
    #[serde(rename = "failure reason")]
    failure_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HttpPeers {
    Compact(#[serde(with = "serde_bytes")] Vec<u8>),
    Dictionary(Vec<HttpPeer>),
}

#[derive(Debug, Deserialize)]
struct HttpPeer {
    ip: String,
    port: u16,
}

fn build_http_announce_url(address: &Url, state: &State, port: u16) -> String {
    let downloaded = state.bytes_complete() as i64;
    let left = state.meta_info.total_length().saturating_sub(downloaded);
    build_http_announce_url_with_values(address, &state.info_hash, downloaded, left, port)
}

fn build_http_announce_url_with_values(address: &Url, info_hash: &[u8], downloaded: i64, left: i64, port: u16) -> String {
    let mut base = address.clone();
    let original_query = base.query().map(str::to_owned);
    base.set_query(None);
    base.set_fragment(None);

    let mut query = original_query.unwrap_or_default();
    append_bytes_query_pair(&mut query, "info_hash", info_hash);
    append_bytes_query_pair(&mut query, "peer_id", &PEER_ID);
    append_query_pair(&mut query, "port", port);
    append_query_pair(&mut query, "uploaded", 0);
    append_query_pair(&mut query, "downloaded", downloaded.max(0));
    append_query_pair(&mut query, "left", left.max(0));
    append_query_pair(&mut query, "compact", 1);
    append_query_pair(&mut query, "numwant", 80);
    append_query_pair(&mut query, "event", "started");

    let mut announce_url = base.to_string();
    announce_url.push('?');
    announce_url.push_str(&query);
    announce_url
}

fn append_query_pair(query: &mut String, key: &str, value: impl Display) {
    if !query.is_empty() {
        query.push('&');
    }
    let _ = write!(query, "{key}={value}");
}

fn append_bytes_query_pair(query: &mut String, key: &str, value: &[u8]) {
    if !query.is_empty() {
        query.push('&');
    }
    let encoded = percent_encode(value, NON_ALPHANUMERIC);
    let _ = write!(query, "{key}={encoded}");
}

fn parse_http_announce_response(bytes: &[u8]) -> TrackerResult<AnnounceResponse> {
    let response: HttpAnnounceResponse = serde_bencode::de::from_bytes(bytes)?;
    if let Some(reason) = response.failure_reason {
        return Err(TrackerError::TrackerFailure(reason));
    }

    let mut peers_addresses = Vec::new();
    if let Some(peers) = response.peers {
        match peers {
            HttpPeers::Compact(bytes) => peers_addresses.extend(parse_compact_ipv4_peers(&bytes)?),
            HttpPeers::Dictionary(peers) => {
                for peer in peers {
                    let ip = peer.ip.parse::<IpAddr>().map_err(|source| TrackerError::InvalidPeerIp {
                        ip: peer.ip.clone(),
                        source,
                    })?;
                    peers_addresses.push(SocketAddr::new(ip, peer.port));
                }
            }
        }
    }
    peers_addresses.extend(parse_compact_ipv6_peers(&response.peers6)?);

    Ok(AnnounceResponse {
        action: 1,
        transaction_id: 0,
        interval: response.interval.unwrap_or(1800).max(1) as i32,
        leechers: response.incomplete.unwrap_or_default().max(0) as i32,
        seeders: response.complete.unwrap_or_default().max(0) as i32,
        peersAddresses: peers_addresses,
    })
}

fn parse_compact_ipv4_peers(bytes: &[u8]) -> TrackerResult<Vec<SocketAddr>> {
    if !bytes.len().is_multiple_of(6) {
        return Err(TrackerError::InvalidCompactPeerList {
            family: "IPv4",
            chunk_size: 6,
            len: bytes.len(),
        });
    }

    Ok(bytes
        .chunks_exact(6)
        .map(|peer| {
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(peer[0], peer[1], peer[2], peer[3])),
                u16::from_be_bytes([peer[4], peer[5]]),
            )
        })
        .collect())
}

fn parse_compact_ipv6_peers(bytes: &[u8]) -> TrackerResult<Vec<SocketAddr>> {
    if !bytes.len().is_multiple_of(18) {
        return Err(TrackerError::InvalidCompactPeerList {
            family: "IPv6",
            chunk_size: 18,
            len: bytes.len(),
        });
    }

    Ok(bytes
        .chunks_exact(18)
        .map(|peer| {
            let mut ip_bytes = [0_u8; 16];
            ip_bytes.copy_from_slice(&peer[..16]);
            SocketAddr::new(IpAddr::V6(Ipv6Addr::from(ip_bytes)), u16::from_be_bytes([peer[16], peer[17]]))
        })
        .collect())
}

///Type of protocol used to connect to the tracker
#[derive(PartialEq, Debug, Clone)]
pub enum TrackerProtocol {
    Udp,
    Http,
}

/// List of all the states that a **UDP or TCP** Tracker can be in
///
/// **TCP and UDP** - Tracker state for both UDP and TCP based tracker
/// **TCP** - Tracker state for only TCP based tracker
/// **UDP** - Tracker state for only UDP based tracker
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TrackerState {
    /// Default state of the Tracker, where **NO** action is performed on the tracker
    /// For : **TCP and UDP** Tracker
    Idle,

    /// The DNS of the tracker is resolving
    /// **TCP and UDP** Tracker
    DNSResolving,

    /// The DNS of the tracker was not resolved on trying to resolve and it shall be tried again
    /// to be resolved after **retry_time** which is usually
    /// 30 secs
    /// TODO : Figure out if it was not resolved because internet was not there
    /// For : **TCP and UDP** Tracker
    DNSUnresolved { retry_time: Instant },

    /// The DNS of the tracker was resolved
    /// For : **TCP and UDP** Tracker
    DNSResolved,

    /// DNS was resolved and a Connect Request was sent to the tracker, for which
    /// we are now waiting to get a response
    /// For : **UDP** Tracker
    WaitingForConnectResponse,

    /// ConnectResponse was received and AnnounceRequest was sent to the tracker, for which
    /// we are now waiting to get a response
    /// For : **UDP** Tracker
    WaitingForAnnounceResponse,

    /// A ScrapeRequest was sent to the tracker, for which we are now watiing to get
    /// a response
    /// For : **UDP** Tracker
    WaitingForScrapeResponse,
}

//impl Display {}
//impl Display for{}
impl Display for TrackerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Idle => write!(f, "Idle"),
            Self::DNSResolving => write!(f, "DNS Resolving"),
            Self::DNSResolved => write!(f, "DNS Resolved"),
            Self::WaitingForConnectResponse => write!(f, "Waiting for Connect Response"),
            Self::WaitingForAnnounceResponse => write!(f, "Waiting for Announce Response"),
            Self::WaitingForScrapeResponse => write!(f, "Waiting for Scrape Response"),
            Self::DNSUnresolved { ref retry_time } => write!(
                f,
                "DNSUnresolved ({:?}/30 sec)",
                Instant::now().duration_since(*retry_time).as_secs()
            ),
        }
    }
}

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
    pub socketAddrs: Arc<Mutex<Vec<SocketAddr>>>,

    /// A sender part of a channel, to send the [Peer] instance to the runDownload()
    /// method of [TorrentFile], created by collecting the socket address of Peers
    /// from the Trackers, received while getting AnnounceResponse or ScrapeResponse
    /// from other trackers.
    pub peer_sender: Arc<UnboundedSender<Peer>>,

    pub udp_channel: Option<UdpTrackerChannel>,

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
    pub tracker_state: AtomicCell<TrackerState>,
}

impl Tracker {
    /// Tries to create a Tracker instance by parsing the given url
    pub fn new(address: &str, torrent_state: Arc<State>, peer_sender: Arc<UnboundedSender<Peer>>) -> Result<Tracker, TrackerError> {
        let address = Url::parse(address)?;

        let connect_request = ArcMutex!(TrackerRequest::None);
        let connect_response = ArcMutex!(TrackerResponse::None);
        let announce_request = ArcMutex!(TrackerRequest::None);
        let announce_response = ArcMutex!(TrackerResponse::None);
        let scrape_request = ArcMutex!(TrackerRequest::None);
        let scrape_response = ArcMutex!(TrackerResponse::None);

        let protocol = match address.scheme() {
            "udp" => TrackerProtocol::Udp,
            "http" | "https" => TrackerProtocol::Http,
            scheme => return Err(TrackerError::UnsupportedProtocol(scheme.to_string())),
        };
        let (sd, rv) = mpsc::unbounded_channel::<Vec<u8>>();

        let udp_channel = match protocol {
            TrackerProtocol::Udp => Some((sd, ArcMutex!(rv))),
            TrackerProtocol::Http => None,
        };

        let tracker_state = ACell!(TrackerState::Idle);

        Ok(Tracker {
            torrent_state,
            address,
            socketAddrs: Arc::default(),
            protocol,
            udp_channel,
            connect_request,
            connect_response,
            announce_request,
            announce_response,
            scrape_request,
            scrape_response,
            peer_sender,
            tracker_state,
        })
    }

    /// Initially we are only given the URL of the tracker, in order to check if the tracker is even
    /// alive or not, we must check if it's IP is availaible or not by simply resolving the
    /// tracker's DNS, that's what this method does, it resolves the DNS of the tracker
    pub async fn resolveTracker(&self) {
        let resolveDNS = || async {
            if let Ok(addrs) = self.address.socket_addrs(|| None) {
                *self.socketAddrs.lock().await = addrs;
                true
            } else {
                false
            }
        };

        self.tracker_state.store(TrackerState::DNSResolving);
        if resolveDNS().await {
            self.tracker_state.store(TrackerState::DNSResolved);
        } else {
            self.tracker_state.store(TrackerState::DNSUnresolved {
                retry_time: Instant::now(),
            });
        }
    }

    // Compares given socket address to the trackers list of socket addresses,
    // if it matches any one of it, then we can say that the given socket address belongs
    // to the tracker i.e the socket address is equal to the Tracker
    pub fn is_udp(&self) -> bool {
        self.protocol == TrackerProtocol::Udp
    }

    pub fn is_http(&self) -> bool {
        self.protocol == TrackerProtocol::Http
    }

    pub async fn isEqualTo(&self, sAdr1: &SocketAddr) -> bool {
        let socket_addresses = self.socketAddrs.lock().await;
        socket_addresses.iter().any(|sAdr2| sAdr1 == sAdr2)
    }

    pub async fn run(&self, _socket: Arc<UdpSocket>) {
        //self.resolveTracker()
    }

    pub async fn run_http(&self) {
        let client = reqwest::Client::new();
        let mut retry_delay = Duration::from_secs(15);

        loop {
            match self.send_http_announce_request(&client).await {
                Ok(response) => {
                    retry_delay = Duration::from_secs(15);
                    let interval = response.interval.max(30) as u64;
                    {
                        let mut announce_response = self.announce_response.lock().await;
                        *announce_response = TrackerResponse::AnnounceResponse(response);
                    }
                    sleep(Duration::from_secs(interval)).await;
                }
                Err(_) => {
                    self.tracker_state.store(TrackerState::DNSUnresolved {
                        retry_time: Instant::now(),
                    });
                    sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(Duration::from_secs(300));
                }
            }
        }
    }

    async fn send_http_announce_request(&self, client: &reqwest::Client) -> TrackerResult<AnnounceResponse> {
        self.tracker_state.store(TrackerState::WaitingForAnnounceResponse);
        let announce_url = build_http_announce_url(&self.address, &self.torrent_state, self.announce_port().await);
        let response_bytes = client.get(announce_url).send().await?.error_for_status()?.bytes().await?;
        let announce_response = parse_http_announce_response(&response_bytes)?;

        for peer_socket_adr in announce_response.peersAddresses.clone() {
            let peer = Peer::new(peer_socket_adr, self.torrent_state.clone());
            let _ = self.peer_sender.send(peer);
        }

        self.tracker_state.store(TrackerState::DNSResolved);
        Ok(announce_response)
    }

    async fn announce_port(&self) -> u16 {
        let ports = self.torrent_state.udp_ports.lock().await;
        ports.first().copied().unwrap_or(6881)
    }

    /// Starts running the tracker
    /// socket => Socket through which the tracker will send UDP request and receive UDP response
    pub async fn run_me(&self, socket: Arc<UdpSocket>) {
        // A timeout duration for all types of responses
        let timeout_duration = |n: u64| Duration::from_secs(15_u64.saturating_mul(1_u64 << n.min(8)));

        let mut no_of_times_connect_request_timeout = 0;

        loop {
            let sendConReq = self.sendConnectRequest(socket.clone());
            match timeout(timeout_duration(no_of_times_connect_request_timeout), sendConReq).await {
                Ok(v) => match v {
                    // TODO : Replace with get_connect_response();
                    Ok(_) => {
                        if let Some(res) = self.getResponse().await {
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
                            let mut no_of_times_announce_request_timeout = 0;

                            'announce: loop {
                                let now = Instant::now();
                                let now = now.duration_since(duration_since_connect_response);

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
                                                                    let _ = self.peer_sender.send(peer);
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
                                                Err(_e) => {
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
                    }
                    Err(_e) => {
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
        let socketAddrs = {
            let socket_addresses = self.socketAddrs.lock().await;
            // TODO : Make choosable among other indices too
            socket_addresses.first().copied().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    "tracker has no resolved socket addresses for connect request",
                )
            })?
        };

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
                let downloaded = self.torrent_state.bytes_complete() as i64;
                let total = self.torrent_state.meta_info.total_length();
                announce_req.set_downloaded(downloaded);
                announce_req.set_uploaded(0);
                announce_req.set_left(total.saturating_sub(downloaded));
                {
                    let ports = self.torrent_state.udp_ports.lock().await;
                    if let Some(port) = ports.first() {
                        announce_req.set_port(*port as i16);
                    }
                }
                announce_req.set_key(rand::random());
            }
        }

        if let Some(announce_req_bytes) = announce_req.serialize_to_bytes() {
            let socketAddrs = {
                let socket_addresses = self.socketAddrs.lock().await;
                // TODO : Make choosable among other indices too
                socket_addresses.first().copied().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::AddrNotAvailable,
                        "tracker has no resolved socket addresses for announce request",
                    )
                })?
            };
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
            DOES_PEER_ALREADY_EXIST = peer.socket_adr == peer_socket_adr;
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
        IS_CONNECT_RESPONSE
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

        IS_ANNOUNCE_RESPONSE
    }

    pub async fn isScrapeResponse(&self, _d: &[u8]) -> bool {
        false // Dummy
    }

    /// Checks from the given buffer, if the given response is an Error or not
    pub async fn isErrorResponse(&self, _d: &[u8]) -> bool {
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

#[derive(Debug, Default)]
pub enum TrackerRequest {
    ConnectRequest(ConnectRequest),
    AnnounceRequest(AnnounceRequest),
    #[default]
    None,
}

#[cfg(test)]
mod tests {
    use super::{
        build_http_announce_url_with_values, parse_compact_ipv4_peers, parse_compact_ipv6_peers, parse_http_announce_response, Tracker,
    };
    use crate::core::state::{DownState, State};
    use bytes::{BufMut, BytesMut};
    use crossbeam::atomic::AtomicCell;
    use hyperblow::parser::torrent_parser::{FileMeta, Info};
    use std::{
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
        sync::Arc,
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::{mpsc, Mutex, RwLock},
    };
    use url::Url;

    #[test]
    fn http_announce_url_percent_encodes_binary_fields() {
        let url = Url::parse("https://tracker.example.test/announce?existing=1").expect("valid URL");
        let info_hash = [
            0x00, 0x01, 0x02, 0x03, 0x04, b'a', b'b', b'c', b'd', b'e', b'f', 0x7f, 0x80, 0x81, 0xfe, 0xff, b'1', b'2', b'3', b'4',
        ];

        let announce_url = build_http_announce_url_with_values(&url, &info_hash, 25, 75, 6881);

        assert!(announce_url.starts_with("https://tracker.example.test/announce?existing=1&"));
        assert!(announce_url.contains("info_hash=%00%01%02%03%04abcdef%7F%80%81%FE%FF1234"));
        assert!(announce_url.contains("peer_id=%2DHBYxxx%2DQMAXYDGHQAHF"));
        assert!(announce_url.contains("downloaded=25"));
        assert!(announce_url.contains("left=75"));
        assert!(announce_url.contains("compact=1"));
    }

    #[test]
    fn parses_http_compact_ipv4_peers() {
        let peers = parse_compact_ipv4_peers(&[127, 0, 0, 1, 0x1a, 0xe1]).expect("valid compact peers");

        assert_eq!(peers, vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881)]);
    }

    #[test]
    fn rejects_malformed_http_compact_ipv4_peers() {
        assert!(parse_compact_ipv4_peers(&[127, 0, 0, 1, 0x1a]).is_err());
    }

    #[test]
    fn parses_http_compact_ipv6_peers() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&Ipv6Addr::LOCALHOST.octets());
        bytes.extend_from_slice(&6881_u16.to_be_bytes());

        let peers = parse_compact_ipv6_peers(&bytes).expect("valid compact IPv6 peers");

        assert_eq!(peers, vec![SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 6881)]);
    }

    #[test]
    fn parses_http_announce_response_with_compact_peers() {
        let mut peers = BytesMut::new();
        peers.put_slice(&[10, 0, 0, 1]);
        peers.put_u16(51413);

        let mut response = BytesMut::new();
        response.put_slice(b"d8:intervali1800e8:completei2e10:incompletei3e5:peers");
        response.put_slice(peers.len().to_string().as_bytes());
        response.put_u8(b':');
        response.put_slice(&peers);
        response.put_u8(b'e');

        let response = parse_http_announce_response(&response).expect("valid HTTP announce response");

        assert_eq!(response.interval, 1800);
        assert_eq!(response.seeders, 2);
        assert_eq!(response.leechers, 3);
        assert_eq!(
            response.peersAddresses,
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 51413)]
        );
    }

    #[test]
    fn parses_http_announce_response_with_dictionary_peers() {
        let response = b"d8:intervali30e5:peersld2:ip9:127.0.0.14:porti6881eeee";

        let response = parse_http_announce_response(response).expect("valid dictionary peer response");

        assert_eq!(
            response.peersAddresses,
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881)]
        );
    }

    #[test]
    fn rejects_http_tracker_failure_response() {
        let response = b"d14:failure reason12:tracker downe";

        assert!(parse_http_announce_response(response).is_err());
    }

    #[tokio::test]
    async fn http_announce_request_reaches_tracker_and_sends_peers_to_channel() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("HTTP tracker should bind");
        let tracker_address = format!("http://{}/announce", listener.local_addr().expect("tracker should have address"));
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("client should connect");
            let mut request = Vec::new();
            let mut buf = [0_u8; 1024];
            loop {
                let read = socket.read(&mut buf).await.expect("request should be readable");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }

            let mut peers = BytesMut::new();
            peers.put_slice(&[127, 0, 0, 1]);
            peers.put_u16(51413);
            let mut body = BytesMut::new();
            body.put_slice(b"d8:intervali30e5:peers");
            body.put_slice(peers.len().to_string().as_bytes());
            body.put_u8(b':');
            body.put_slice(&peers);
            body.put_u8(b'e');

            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            socket.write_all(response.as_bytes()).await.expect("response header should write");
            socket.write_all(&body).await.expect("response body should write");

            String::from_utf8(request).expect("request should be UTF-8")
        });

        let (peer_sender, mut peer_receiver) = mpsc::unbounded_channel();
        let tracker = Tracker::new(&tracker_address, test_state(vec![3; 20]), Arc::new(peer_sender)).expect("tracker should construct");

        let response = tracker
            .send_http_announce_request(&reqwest::Client::new())
            .await
            .expect("HTTP announce should succeed");

        assert_eq!(
            response.peersAddresses,
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 51413)]
        );
        let peer = peer_receiver.recv().await.expect("peer should be sent to download channel");
        assert_eq!(peer.socket_adr, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 51413));

        let request = server.await.expect("server should finish");
        assert!(request.starts_with("GET /announce?"));
        assert!(request.contains("info_hash=%03%03%03%03"));
        assert!(request.contains("compact=1"));
    }

    fn test_state(info_hash: Vec<u8>) -> Arc<State> {
        Arc::new(State {
            meta_info: FileMeta {
                announce: "http://tracker.example.test/announce".to_string(),
                announce_list: None,
                info: Info {
                    name: Some("tracker-test".to_string()),
                    length: Some(1024),
                    files: None,
                    piece_length: Some(16 * 1024),
                    pieces: Vec::new(),
                },
                creation_data: None,
                comment: None,
                encoding: None,
                created_by: None,
                acceptable_source: None,
            },
            d_state: DownState::Unknown,
            file_tree: None,
            trackers: Arc::new(RwLock::new(Vec::new())),
            udp_ports: Arc::new(Mutex::new(vec![6881])),
            tcp_ports: Arc::new(Mutex::new(Vec::new())),
            info_hash,
            pieces_hash: Vec::new(),
            peers: Arc::new(Mutex::new(Vec::new())),
            uptime: AtomicCell::new(0),
            bytes_complete: AtomicCell::new(0),
            pieces_downloaded: AtomicCell::new(0),
        })
    }
}
