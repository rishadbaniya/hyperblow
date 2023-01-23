use crate::ArcMutex;

use super::state::State;
use super::tracker::Tracker;
use std::time::Duration;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
    time::{sleep, timeout},
};

/// PeerState denotes high level overview of the
/// current state of relationship with the Peer
#[derive(Debug, Clone)]
pub enum PeerState {
    /// Haven't even tried to connect to the peer
    NotConnected,

    /// Trying to connect to the peer
    TryingToConnect,

    /// Staying idle, because Connection timeout occured while trying to connect with the peer
    ConnectionTimeoutIdle,

    /// Staying idle, because TcpStream error occured while trying to connect with the peer
    ConnectionErrorIdle,

    /// Connected with the peer
    Connected,
    // TODO: Add more states later on
    //RequestingPiece,
    // DOWNLOADING_PIECE,
}

/// It defines the type of Peer
#[derive(Debug, Clone)]
pub enum PeerType {
    /// One that doesnt have all pieces and wants more piece
    Leecher,

    /// One that has all the needed pieces
    Seeder,

    /// One that has downloaded required files and doesnt wanna download other files
    PartialSeeder,

    /// PeerType not figured out
    Unknown,
}

/// PeerInfo holds crucial informations about the Peer, such as the pieces the peer has
/// or doesn't have, the type of the peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Zero based index of the piece the peer has
    pieces_have: Vec<u32>,

    /// Zero based index of the piece the peer does not have
    pieces_not_have: Vec<u32>,

    /// Whether the peer is a Seeder or Leecher
    peer_type: PeerType,

    /// State of the peer
    peer_state: PeerState,
}

#[derive(Debug)]
pub struct Peer {
    /// An Owned Read Split Half of the connected TcpStream
    tcp_read_half: Arc<Mutex<Option<OwnedReadHalf>>>,

    /// An Owned Write Split Half of the connected TcpStream
    tcp_write_half: Arc<Mutex<Option<OwnedWriteHalf>>>,

    /// Holds the information and state of the Peer
    pub info: Arc<Mutex<PeerInfo>>,

    /// State of this torrent session
    state: Arc<State>,

    /// The socket address of the peer
    pub socket_adr: SocketAddr,
}

impl Peer {
    /// Creates a new peer instance with the given socket address of the peer
    ///
    /// socket_adr : The Socket Address of the peer we're trying to connect with
    /// state : The State of teh torrent session
    pub fn new(socket_adr: SocketAddr, state: Arc<State>) -> Self {
        let info = ArcMutex!(PeerInfo {
            pieces_have: Vec::new(),
            pieces_not_have: Vec::new(),
            peer_type: PeerType::Unknown,
            peer_state: PeerState::NotConnected,
        });

        let tcp_read_half = ArcMutex!(None);
        let tcp_write_half = ArcMutex!(None);

        Self {
            tcp_read_half,
            tcp_write_half,
            info,
            state,
            socket_adr,
        }
    }

    /// It will run infinitely, non blockingly, until it gets a TCP connection with the given
    ///
    /// socket address
    ///
    /// A 16 seconds of connection timeout time is kept to make a reliable
    /// TCP Connection with the peer.
    ///
    /// A higher connection timeout time could be added too, but even if we get a
    /// TCP Connection keeping the timeout higher, the connection won't be
    /// reliable enough to exchange pieces with the peer.
    ///
    /// TODO : Figure out the sleep duration for connection timeout
    async fn connect(&self, socket_adr: SocketAddr) {
        let CONNECTION_TIMEOUT = Duration::from_secs(16);

        loop {
            match timeout(CONNECTION_TIMEOUT, TcpStream::connect(socket_adr)).await {
                Ok(connection) => match connection {
                    Ok(tcp_stream) => {
                        let (tcp_read_half, tcp_write_half) = tcp_stream.into_split();
                        (*self.tcp_write_half.lock().await) = Some(tcp_write_half);
                        (*self.tcp_read_half.lock().await) = Some(tcp_read_half);
                    }
                    Err(_) => {
                        // Err while trying to achieve a TCP Connection with the peer
                        // TODO : Handle Connection timeout properly with
                        // proper protocol implementation rather than this 1000 secs of sleep
                        sleep(Duration::from_secs(1000));
                    }
                },
                Err(_) => {
                    // TCP Connection timeout
                    // TODO : Handle Connection timeout properly with
                    // proper protocol implementation rather than this 1000 secs of sleep
                    sleep(Duration::from_secs(1000));
                }
            }
        }
    }

    /// Creates and sends a HANDSHAKE message to the peer and returns the
    /// responses of that Handshake Message
    ///
    /// TODO : Write some stuff about handshake message and its response
    ///
    /// NOTE : It drops the connection as soon as it sees CHOKE message as a response
    /// of the HANDSHAKE message
    ///
    async fn send_handshake_message(&self) {
        //const HANDSHAKE_RESPONSE_WAIT_TIME: u64 = 2;

        //// Creates a HANDSHAKE Message
        //let mut handshake_msg = Handshake::default();
        //let lock_details = self.details.lock().await;
        //let info_hash = lock_details.info_hash.as_ref().unwrap().clone();
        //handshake_msg.set_info_hash(info_hash.to_vec());
        //drop(lock_details);

        //// Writes the HANDSHAKE message on the TCPStream
        //self.write_half.write_all(&handshake_msg.getBytesMut()).await;

        //// Waits for all the messages that peer is gonna send as response to the HANDSHAKE message we sent
        //let mut messages = Vec::new();
        //if let Some(mut msgs) = self.receiver.recv().await {
        //    messages.append(&mut msgs);
        //    // Store all responses sent after 2 seconds of receiving HANDSHAKE response, its usually BITFIELD/HAVE/EXTENDED
        //    timeout(Duration::from_secs(HANDSHAKE_RESPONSE_WAIT_TIME), async {
        //        loop {
        //            if let Some(mut _msgs) = self.receiver.recv().await {
        //                messages.append(&mut _msgs);
        //            }
        //        }
        //    })
        //    .await;
        //}

        //// If the peer sends CHOKE, then we'll disconnect from that peer
        //if messages.contains(&Message::CHOKE) {
        //    self.write_half.shutdown();
        //}
        //Ok(messages)
    }

    ///
    /// TODO : Write some stuff about INTERESTED message and its response
    ///
    /// Creates and sends INTERESTED message to the peer
    pub async fn sendInterestedMessage(&mut self) {
        // self.write_half.write_all(&Interested::build_message()).await;

        // self.receiver.recv().await
    }
}
