mod codec;
mod messages;
mod piece;

use super::state::State;
use crate::ArcMutex;
use codec::{PeerCodecError, PeerMessageCodec};
use futures_util::{SinkExt, StreamExt};
use messages::{Handshake, Message};
use std::{io, net::SocketAddr, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    sync::Mutex,
    time::{sleep, timeout},
};

use tokio_util::codec::Framed;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(16);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(2);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);

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
    HandshakeComplete,
    Running,
    Disconnected,
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Error)]
enum PeerError {
    #[error("timed out connecting to peer {0}")]
    ConnectionTimeout(SocketAddr),

    #[error("failed to connect to peer {addr}")]
    Connection { addr: SocketAddr, source: io::Error },

    #[error("peer codec error")]
    Codec(#[from] PeerCodecError),

    #[error("peer closed before handshake")]
    HandshakeClosed,

    #[error("peer sent non-handshake message first: {0:?}")]
    UnexpectedHandshakeMessage(Message),

    #[error("peer handshake info hash did not match torrent")]
    InfoHashMismatch,
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
    pub async fn run(&self) {
        let mut retry_delay = INITIAL_RETRY_DELAY;
        loop {
            match self.run_session().await {
                Ok(()) => {
                    self.set_peer_state(PeerState::Disconnected).await;
                    break;
                }
                Err(_) => {
                    self.set_peer_state(PeerState::ConnectionErrorIdle).await;
                    sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
                }
            }
        }
    }

    async fn run_session(&self) -> Result<(), PeerError> {
        let mut stream = self.connect_once().await?;
        self.send_and_validate_handshake(&mut stream).await?;
        stream.send(vec![Message::Interested]).await?;
        self.set_peer_state(PeerState::Running).await;

        while let Some(message) = stream.next().await {
            self.handle_message(message?).await;
        }

        Ok(())
    }

    async fn connect_once(&self) -> Result<Framed<TcpStream, PeerMessageCodec>, PeerError> {
        self.set_peer_state(PeerState::TryingToConnect).await;
        let tcp_stream = match timeout(CONNECTION_TIMEOUT, TcpStream::connect(self.socket_adr)).await {
            Ok(Ok(tcp_stream)) => tcp_stream,
            Ok(Err(source)) => {
                self.set_peer_state(PeerState::ConnectionErrorIdle).await;
                return Err(PeerError::Connection {
                    addr: self.socket_adr,
                    source,
                });
            }
            Err(_) => {
                self.set_peer_state(PeerState::ConnectionTimeoutIdle).await;
                return Err(PeerError::ConnectionTimeout(self.socket_adr));
            }
        };

        self.set_peer_state(PeerState::Connected).await;
        Ok(Framed::new(tcp_stream, PeerMessageCodec))
    }

    async fn send_and_validate_handshake(&self, stream: &mut Framed<TcpStream, PeerMessageCodec>) -> Result<(), PeerError> {
        stream.send(vec![Message::Handshake(Handshake::new(self.state.clone()))]).await?;
        self.set_peer_state(PeerState::SentHandshake).await;

        let message = timeout(HANDSHAKE_TIMEOUT, stream.next())
            .await
            .map_err(|_| PeerError::ConnectionTimeout(self.socket_adr))?
            .ok_or(PeerError::HandshakeClosed)??;

        match message {
            Message::Handshake(handshake) if handshake.info_hash() == self.state.info_hash.as_slice() => {
                self.set_peer_state(PeerState::HandshakeComplete).await;
                Ok(())
            }
            Message::Handshake(_) => Err(PeerError::InfoHashMismatch),
            message => Err(PeerError::UnexpectedHandshakeMessage(message)),
        }
    }

    async fn handle_message(&self, message: Message) {
        match message {
            Message::Have(have) => {
                let mut info = self.info.lock().await;
                if !info.pieces_have.contains(&have.piece_index) {
                    info.pieces_have.push(have.piece_index);
                }
            }
            Message::Bitfield(bitfield) => {
                let mut info = self.info.lock().await;
                info.pieces_have = bitfield.have.into_iter().map(|piece| piece as u32).collect();
                info.pieces_not_have = bitfield.not_have.into_iter().map(|piece| piece as u32).collect();
                info.peer_type = if info.pieces_not_have.is_empty() {
                    PeerType::Seeder
                } else {
                    PeerType::Leecher
                };
            }
            _ => {}
        }
    }

    async fn set_peer_state(&self, peer_state: PeerState) {
        let mut info = self.info.lock().await;
        info.peer_state = peer_state;
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

#[cfg(test)]
mod tests {
    use super::{Peer, PeerError};
    use crate::core::state::{DownState, State};
    use crossbeam::atomic::AtomicCell;
    use hyperblow::parser::torrent_parser::{FileMeta, Info};
    use std::sync::Arc;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::{Mutex, RwLock},
    };

    #[tokio::test]
    async fn peer_session_sends_handshake_and_interested() {
        let info_hash = vec![7; 20];
        let state = test_state(info_hash.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener should bind");
        let address = listener.local_addr().expect("listener should have local address");

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("peer should connect");
            let mut handshake = [0_u8; 68];
            socket.read_exact(&mut handshake).await.expect("peer should send handshake");
            assert_eq!(handshake[0], 19);
            assert_eq!(&handshake[1..20], b"BitTorrent protocol");
            assert_eq!(&handshake[28..48], info_hash.as_slice());

            socket.write_all(&handshake).await.expect("server should send handshake response");

            let mut interested = [0_u8; 5];
            socket
                .read_exact(&mut interested)
                .await
                .expect("peer should send interested message");
            assert_eq!(interested, [0, 0, 0, 1, 2]);
        });

        let peer = Peer::new(address, state);
        peer.run_session().await.expect("peer session should complete after server closes");
        server.await.expect("server task should complete");
    }

    #[tokio::test]
    async fn peer_session_rejects_wrong_info_hash() {
        let state = test_state(vec![7; 20]);
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener should bind");
        let address = listener.local_addr().expect("listener should have local address");

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("peer should connect");
            let mut handshake = [0_u8; 68];
            socket.read_exact(&mut handshake).await.expect("peer should send handshake");
            handshake[28..48].copy_from_slice(&[9; 20]);
            socket.write_all(&handshake).await.expect("server should send mismatched handshake");
        });

        let peer = Peer::new(address, state);
        let error = peer.run_session().await.expect_err("mismatched info hash should fail");

        assert!(matches!(error, PeerError::InfoHashMismatch));
        server.await.expect("server task should complete");
    }

    fn test_state(info_hash: Vec<u8>) -> Arc<State> {
        Arc::new(State {
            meta_info: FileMeta {
                announce: "udp://tracker.example.test:6969".to_string(),
                announce_list: None,
                info: Info {
                    name: Some("peer-test".to_string()),
                    length: Some(0),
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
