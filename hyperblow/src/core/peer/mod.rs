mod codec;
mod messages;
mod piece;

use super::state::State;
use crate::ArcMutex;
use messages::Message;
use std::time::Duration;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpStream,
    sync::Mutex,
    time::{sleep, timeout},
};

use codec::PeerMessageCodec;
use tokio_util::codec::Framed;

/// PeerState denotes high level overview of the current state of
/// relationship of this client with the remote Peer
#[derive(Debug, Clone)]
pub enum PeerState {
    /// Haven't even made a TCP Connection
    NotConnected,

    /// Trying to make a TCP Connection
    TryingToConnect,

    /// Staying idle, because Connection timeout occured while trying to make a TCP Connection
    /// with the peer
    ConnectionTimeoutIdle,

    /// Staying idle, because TcpStream error occured while trying to make a TCP Connection  with
    /// the peer
    ConnectionErrorIdle,

    /// Made a TCP Connection with the peer
    Connected,

    /// Sent a Handshake to the Peer
    SentHandshake,
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
    ///// An Owned Read Split Half of the connected TcpStream
    //tcp_read_half: Arc<Mutex<Option<OwnedReadHalf>>>,

    ///// An Owned Write Split Half of the connected TcpStream
    //tcp_write_half: Arc<Mutex<Option<OwnedWriteHalf>>>,
    /// Holds the information and state of the Peer
    pub info: Arc<Mutex<PeerInfo>>,

    /// State of this torrent session
    state: Arc<State>,

    /// The socket address of the peer
    pub socket_adr: SocketAddr,

    stream: Arc<Mutex<Option<Framed<TcpStream, PeerMessageCodec>>>>,
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

        let stream = ArcMutex!(None);

        Self {
            info,
            state,
            socket_adr,
            stream,
        }
    }

    /// It will run infinitely, non blockingly, until it gets a TCP connection with the given
    /// socket address
    ///
    /// A 16 seconds of connection timeout time is kept to make a reliable TCP Connection
    /// with the peer.
    ///
    /// A higher connection timeout time could be added too, but even if we get a TCP
    /// Connection keeping the timeout higher, the connection won't be reliable enough
    /// to exchange pieces with the peer.
    ///
    /// TODO : Figure out the sleep duration for connection timeout or socket error, i.e after a
    /// socket error or connection timeout, figure out the time until next connection attempt
    async fn connect(&self, socket_adr: SocketAddr) {
        let CONNECTION_TIMEOUT = Duration::from_secs(16);
        loop {
            match timeout(CONNECTION_TIMEOUT, TcpStream::connect(socket_adr)).await {
                Ok(connection) => match connection {
                    Ok(tcp_stream) => {
                        let peer_message_codec = codec::PeerMessageCodec;
                        let codec_stream = Framed::new(tcp_stream, peer_message_codec);
                        let mut stream = self.stream.lock().await;
                        *stream = Some(codec_stream);
                    }
                    Err(_) => {
                        // Err while trying to achieve a TCP Connection with the peer
                        // TODO : Handle Connection timeout properly with
                        // proper protocol implementation rather than this 1000 secs of sleep
                        sleep(Duration::from_secs(1000)).await;
                    }
                },
                Err(_) => {
                    // TCP Connection timeout
                    // TODO : Handle Connection timeout properly with
                    // proper protocol implementation rather than this 1000 secs of sleep
                    sleep(Duration::from_secs(1000)).await;
                }
            }
        }
    }

    /// It's a required and first message sent to a peer after creating a TCP connection
    /// with the peer
    ///
    /// A Handshake is (49 + len(pstr)) bytes long
    ///
    /// Creates and sends a Handshake message to the peer and returns the
    /// responses of that Handshake Message
    ///
    /// NOTE : This method should only be called after calling self.connect() method
    /// i.e after successfully establishing a TCP Connection with the peer
    async fn send_handshake_message(&self) {
        // Steps :
        //
        // 1. Send the Handshake message to the peer
        // 2. Wait for messages that the peer is gonna send to us
        // 3. After receiving the messages
        //const HANDSHAKE_RESPONSE_WAIT_TIME: u64 = 2;

        //// Creates a Handshake Message
        //let handshake_message = Handshake::new(self.state.clone()).to_bytes();

        //let write_half = write_half_lock.as_mut().unwrap();
        //let read_half = read_half_lock.as_mut().unwrap();

        //// Waits for all the messages that peer is gonna send as response
        //// to the Handshake message we sent

        //// A 4 Kb buffer for the response of Handshake message
        //// TODO: Find the perfect buffer size

        ////let mut messages = Vec::new();
        ////    messages.append(&mut msgs);
        ////    //    // Store all responses sent after 2 seconds of receiving HANDSHAKE response, its usually BITFIELD/HAVE/EXTENDED
        ////    //    timeout(Duration::from_secs(HANDSHAKE_RESPONSE_WAIT_TIME), async {
        ////    //        loop {
        ////    //            if let Some(mut _msgs) = self.receiver.recv().await {
        ////    //                messages.append(&mut _msgs);
        ////    //            }
        ////    //        }
        ////    //    })
        ////    //    .await;

        ////// If the peer sends CHOKE, then we'll disconnect from that peer
        ////if messages.contains(&Message::CHOKE) {
        ////    self.write_half.shutdown();
        ////}
        ////Ok(messages)
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
