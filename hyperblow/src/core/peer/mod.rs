mod block;
mod messages;
mod piece;

use self::messages::Cancel;
use self::messages::Have;

use super::state::State;
use crate::ArcMutex;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use messages::Handshake;
use messages::Message;
use std::os::unix::net::Messages;
use std::time::Duration;
use std::{net::SocketAddr, sync::Arc};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::{
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
    time::{sleep, timeout},
};
use tokio_util::codec::{Decoder, Encoder};

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
        const HANDSHAKE_RESPONSE_WAIT_TIME: u64 = 2;

        // Creates a Handshake Message
        let handshake_message = Handshake::new(self.state.clone()).to_bytes();

        let mut read_half_lock = self.tcp_read_half.lock().await;
        let mut write_half_lock = self.tcp_write_half.lock().await;

        let write_half = write_half_lock.as_mut().unwrap();
        let read_half = read_half_lock.as_mut().unwrap();

        write_half.write_all(&handshake_message);

        // Waits for all the messages that peer is gonna send as response
        // to the Handshake message we sent

        // A 4 Kb buffer for the response of Handshake message
        // TODO: Find the perfect buffer size
        let mut buf = [0; 4096];
        //let mut messages = Vec::new();
        if let Ok(d) = read_half.read(&mut buf).await {
            //    messages.append(&mut msgs);
            //    //    // Store all responses sent after 2 seconds of receiving HANDSHAKE response, its usually BITFIELD/HAVE/EXTENDED
            //    //    timeout(Duration::from_secs(HANDSHAKE_RESPONSE_WAIT_TIME), async {
            //    //        loop {
            //    //            if let Some(mut _msgs) = self.receiver.recv().await {
            //    //                messages.append(&mut _msgs);
            //    //            }
            //    //        }
            //    //    })
            //    //    .await;
        }

        //// If the peer sends CHOKE, then we'll disconnect from that peer
        //if messages.contains(&Message::CHOKE) {
        //    self.write_half.shutdown();
        //}
        //Ok(messages)
    }

    async fn get_messages(&self) {
        // It's the max amount of data we'll ever receive, which is the max size of block we're
        // ever gonna request
        const MAX_BUFFER_CAPACITY: usize = 16000;

        let mut messages: Vec<Message> = Vec::new();
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

use bytes::{BufMut, BytesMut};
struct PeerMessageCodec {
    messages: Vec<Message>,
}

impl PeerMessageCodec {
    // Checks if the given bytes frame is Keep Alive Message or not
    pub fn is_bytes_keep_alive(&self, src: &BytesMut) -> bool {
        if src.len() == 4 {
            let length_prefix_bytes = &src[0..=3];
            if let Ok(length_prefix) = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes) {
                length_prefix == 0;
            }
        }
        false
    }

    pub fn is_bytes_handshake(&self, src: &BytesMut) -> bool {
        let pstr = String::from("BitTorrent protocol");
        if src.len() >= 68 {
            let pstr_len = src[0];
            if pstr_len == 19 {
                return true;
                //let pstr_bytes = &[1..=19];
                //let src_pstr = String::from_utf8(pstr_bytes);
                // TODO : Check all the fields such as pstr, reserved bytes, info hash of the
                // Handshake Message
            }
        }
        return false;
    }
}

impl Decoder for PeerMessageCodec {
    type Item = Vec<Message>;
    type Error = std::io::Error;
    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        return if src.len() == 0 {
            Ok(Some(self.messages))
        } else if self.is_bytes_keep_alive(src) {
            src.split_to(4);
            self.messages.push(Message::KeepAlive);
            Ok(None)
        } else if self.is_bytes_handshake(src) {
            src.split_to(68);
            Ok(None)
        } else if src.len() >= 5 {
            let length_prefix_bytes = &src[0..=3];
            if let Ok(length_prefix) = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes) {
                let message_id = src[4];
                if length_prefix == 1 {
                    match message_id {
                        0 => self.messages.push(Message::Choke),
                        1 => self.messages.push(Message::Unchoke),
                        2 => self.messages.push(Message::Interested),
                        3 => self.messages.push(Message::NotInterested),
                    }
                    src.split_to(5);
                } else if length_prefix == 5 && message_id == 4 {
                    self.messages.push(Message::Have(Have::from_bytes(src)));
                    src.split_to(9);
                } else if length_prefix > 1 && message_id == 5 {
                } else if length_prefix == 13 && message_id == 6 {
                } else if length_prefix == 13 && message_id == 7 {
                    // Piece Message
                } else if length_prefix == 13 && message_id == 8 && src.len() == 17 {
                    // Cancel Message
                    let cancel_message = Message::Cancel(Cancel::from_bytes(src));
                    self.messages.push(cancel_message);
                    src.split_to(17);
                } else if length_prefix == 3 && message_id == 9 && src.len() == 7 {
                    // Port Message
                    let port_bytes = &src[5..=6];
                    let port = ReadBytesExt::read_u16::<BigEndian>(&mut port_bytes).unwrap();
                    self.messages.push(Message::Port(port));
                    src.split_to(7);
                }
            }
            Ok(None)
        } else {
            //     src.len() == 4 {
            //      // TODO : Check if the length is (0_u32) as well, coz a block's remaing data can also be 4
            //      // bytes
            //      bytes.split_to(4);
            //      Some(Ok(Message::KEEP_ALIVE))
            //      // Ok(None) means that more data is needed to build a Message Frame
            Ok(None)
        };
    }
}

//    if bytes.len() == 0 {
//        // If the buffer is empty then it means there is no message
//        None
//    } else if bytes.len() == 4 {
//        // TODO : Check if the length is (0_u32) as well, coz a block's remaing data can also be 4
//        // bytes
//        bytes.split_to(4);
//        Some(Ok(Message::KEEP_ALIVE))
//    } else {
//        let pstr_len = bytes[0];
//        if pstr_len == 19u8 {
//            Some(Ok(Message::HANDSHAKE(Handshake::from(bytes))))
//        } else {
//            let mut message_id = 100;
//            if let Some(v) = bytes.get(4) {
//                message_id = *v;
//            }
//            match message_id {
//                5 => {
//                    return Some(match Bitfield::from(bytes) {
//                        Ok(bitfield) => Ok(Message::BITFIELD(bitfield)),
//                        Err(e) => Err(e),
//                    });
//                }
//                6 => Some(Ok(Message::REQUEST)),
//                7 => {
//                    return Some(match Block::from(bytes) {
//                        Ok(block) => Ok(Message::PIECE(block)),
//                        Err(e) => Err(e),
//                    });
//                }
//                8 => {
//                    bytes.split_to(5);
//                    Some(Ok(Message::CANCEL))
//                }
//                9 => {
//                    bytes.split_to(5);
//                    Some(Ok(Message::PORT))
//                }
//                20 => Some(Ok(Message::EXTENDED(Extended::from(bytes)))),
//                _ => None,
//            }
//        }
//    }
//
