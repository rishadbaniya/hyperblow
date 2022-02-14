#![allow(non_camel_case_types)]
#![allow(unused_must_use)]
#![allow(dead_code)]

use super::{start::__Details, Bitfield, Block, Extended, Handshake, Have, Interested, Message, Unchoke};
use crate::work::Request;
use crate::Result;
use bytes::BytesMut;
use futures::{select, FutureExt};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{
    tcp::{ReadHalf, WriteHalf},
    TcpStream,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};

/// Current relationship with the Peer
pub enum PeerStatus {
    NOT_CONNECTED,
    CONNECTED,
    SENT_HANDHSAKE,
}

/// Type of Peer
pub enum PeerType {
    /// One that doesnt have all pieces and wants more piece
    LEECHER,

    /// One that has all the needed pieces
    SEEDER,

    /// One that has downloaded required files and doesnt wanna download other files
    PARTIAL_SEEDER,
}

/// PeerInfo holds crucial informations about the Peer such as
/// the pieces the peer has or doesn't have, if the peer is a Seeder or a Leecher
///
/// It should be created when the peer sends us some messages like BITFIELD, EXTENDED or HAVE
pub struct PeerInfo {
    /// Zero based index of the piece the peer has
    pieces_have: Vec<u32>,

    /// Zero based index of the piece the peer does not have
    pieces_not_have: Vec<u32>,

    /// Whether the peer is a Seeder or Leecher
    peer_type: PeerType,
}

pub struct Peer<'a> {
    /// Current state of relationship between us and the peer
    status: Option<Arc<RwLock<PeerStatus>>>,

    /// Holds information needed to download pieces
    info: Option<Arc<RwLock<PeerInfo>>>,

    /// TcpStream after getting connected to the peer
    tcp_stream: Option<TcpStream>,

    /// Wrapper around WriteHalf to send certain Bittorent Message as raw bytes to the peer
    tcp_sender: Option<TCPSender<'a>>,

    /// Wrapper around ReadHalf to receive raw bytes as certain Bittorent Message from the peer
    tcp_receiver: Option<TCPReceiver<'a>>,

    /// Socket address of the Peer
    socket_adr: SocketAddr,

    /// Details of the torrent file
    details: __Details,
}

impl<'a> Peer<'a> {
    /// Creates a new peer instance by storing the socket address of the
    /// peer
    pub fn new(socket_adr: SocketAddr, details: __Details) -> Self {
        let status = Some(Arc::new(RwLock::new(PeerStatus::NOT_CONNECTED)));
        Self {
            status,
            info: None,
            tcp_stream: None,
            tcp_sender: None,
            tcp_receiver: None,
            socket_adr,
            details,
        }
    }

    /// Tries to connect to the peer until CONNECTION_TIMEOUT or until some ERROR occurs
    ///
    /// It'll return Error if trying to connect fails, on such condition we should try to connect
    /// to the peer again after X seconds, where X is any time defined by Developer
    pub async fn try_connect(&'a mut self) -> Option<((TCPSender<'a>, TCPReceiver<'a>), UnboundedSender<Vec<Message>>)> {
        let CONNECTION_TIMEOUT = Duration::from_secs(60);

        match timeout(CONNECTION_TIMEOUT, TcpStream::connect(self.socket_adr)).await {
            Ok(Ok(tcp_stream)) => {
                self.tcp_stream = Some(tcp_stream);
                let (channel_sender, channel_receiver) = unbounded_channel::<Vec<Message>>();
                let (read_half, write_half) = self.tcp_stream.as_mut().unwrap().split();

                let tcp_receiver = TCPReceiver::new(read_half);
                let tcp_sender = TCPSender::new(write_half, self.details.clone(), channel_receiver);

                // Changes the PeerStatus to CONNECTED
                let mut write_lock_status = self.status.as_ref().unwrap().write().await;
                *write_lock_status = PeerStatus::CONNECTED;

                return Some(((tcp_sender, tcp_receiver), channel_sender));
            }
            _ => {
                return None;
            }
        }
    }
}

/// Peer Request (TCP) :
///
/// OBJECTIVE : Connect to Peers and download pieces block by block
/// We do it by making a TCP connection with the "peer"
///
/// NOTE : A torrent contains multiple pieces and pieces contain multiple blocks, each block
/// request should not be greater than 16Kb (16384 bytes), i.e we should not request block
/// greater than 16Kb(16384 bytes) in request message
///
pub async fn peer_request(socket_adr: SocketAddr, details: __Details) {
    const CONNECTION_TIMEOUT: u64 = 60;
    const CONNECTION_FAILED_TRY_AGAIN_AFTER: u64 = 120;
    const MAX_BUFFER_CAPACITY: u32 = 16384;

    let PIECE_LENGTH = details.lock().await.piece_length.unwrap() as u32;
    loop {
        let mut peer = Peer::new(socket_adr, details.clone());
        match peer.try_connect().await {
            Some(((mut tcp_sender, mut tcp_receiver), channel_sender)) => {
                // Continuosly reads on the stream for some message
                let read = async move {
                    loop {
                        if let Ok(msgs) = tcp_receiver.getMessage().await {
                            channel_sender.send(msgs);
                        } else {
                            break;
                        }
                    }
                };

                let write = async move {
                    let mut messages: Vec<Message> = Vec::new();

                    // Sends HANDSHAKE message
                    let mut handshake_response = tcp_sender.sendHandshakeMessage().await.unwrap();
                    messages.append(&mut handshake_response);

                    // Sends INTERESTED message
                    let mut interested_response = tcp_sender.sendInterestedMessage().await.unwrap();
                    messages.append(&mut interested_response);

                    // Sends UNCHOKE message
                    tcp_sender.sendUnchokeMessage();
                    println!("{:?}", messages);
                };

                // End both the future as soon as one gets completed
                select! {
                    () = read.fuse() => (),
                    () = write.fuse() => ()
                };
            }
            _ => {}
        };

        sleep(Duration::from_secs(260)).await;
    }

    //                     let peer_details = PeerDetails::from(&mut messages);

    //                     for piece_index in peer_details.pieces_have {
    //                         let mut byte_index: u32 = 0;
    //                         let mut blocks: Vec<Block> = Vec::new();

    //                         let x = Instant::now();
    //                         loop {
    //                             if PIECE_LENGTH != byte_index {
    //                                 let length = {
    //                                     if PIECE_LENGTH - byte_index < MAX_BUFFER_CAPACITY {
    //                                         PIECE_LENGTH - byte_index
    //                                     } else {
    //                                         MAX_BUFFER_CAPACITY
    //                                     }
    //                                 };

    //                                 tcp_sender.write_half.write_all(&Request::build_message(piece_index, byte_index, length)).await;

    //                                 if let Some(msg) = tcp_sender.receiver.recv().await {
    //                                     if let Message::PIECE(block) = &msg[0] {
    //                                         blocks.push(block.clone());
    //                                         byte_index = block.byte_index + block.raw_block.len() as u32;
    //                                     }
    //                                 }
    //                             } else {
    //                                 println!(
    //                                     "DOWNLOADED total {} blocks of index {} in {:?}",
    //                                     blocks.len(),
    //                                     piece_index,
    //                                     Instant::now().duration_since(x),
    //                                 );
    //                                 break;
    //                             }
    //                         }
    //                     }
    //                 };
}

#[derive(Debug, Clone)]
struct PeerDetails {
    pieces_have: Vec<u32>,
    pieces_not_have: Vec<u32>,
}

impl PeerDetails {
    fn from(v: &mut Vec<Message>) -> Self {
        let mut pieces_have: Vec<u32> = Vec::new();
        let mut pieces_not_have: Vec<u32> = Vec::new();
        *v = v
            .iter()
            .filter(|f| {
                match f {
                    Message::BITFIELD(_bitfield) => {
                        pieces_have.append(&mut _bitfield.have.clone());
                        pieces_not_have.append(&mut _bitfield.not_have.clone());
                    }
                    Message::HAVE(_have) => {
                        pieces_have.push(_have.piece_index);
                    }
                    _ => {
                        return true;
                    }
                }
                return false;
            })
            .map(|v| v.clone())
            .collect();

        Self { pieces_have, pieces_not_have }
    }
}

/// A function that removes the bytes of that message from buffer
/// that it provided after it finds a message
async fn messageHandler<'a>(bytes: &mut BytesMut, receiver: &mut TCPReceiver<'a>) -> Option<crate::Result<Message>> {
    if bytes.len() == 0 {
        // If the buffer is empty then it means there is no message
        None
    } else if bytes.len() == 4 {
        // TODO : Check if the length is (0_u32) as well
        Some(Ok(Message::KEEP_ALIVE))
    } else {
        // If it's a HANDSHAKE message, then the first message is pstr_len, whose value is 19
        // TODO : Check if pstr = "BitTorrent protocol" as well
        let pstr_len = bytes[0];

        if pstr_len == 19u8 {
            Some(Ok(Message::HANDSHAKE(Handshake::from(bytes))))
        } else {
            let mut message_id = 100;
            if let Some(v) = bytes.get(4) {
                message_id = *v;
            }
            match message_id {
                0 => {
                    bytes.split_to(5);
                    Some(Ok(Message::CHOKE))
                }
                1 => {
                    bytes.split_to(5);
                    Some(Ok(Message::UNCHOKE))
                }
                2 => {
                    bytes.split_to(5);
                    Some(Ok(Message::INTERESTED))
                }
                3 => {
                    bytes.split_to(5);
                    Some(Ok(Message::NOT_INTERESTED))
                }
                4 => Some(Ok(Message::HAVE(Have::from(bytes)))),
                5 => {
                    return Some(match Bitfield::from(bytes) {
                        Ok(bitfield) => Ok(Message::BITFIELD(bitfield)),
                        Err(e) => Err(e),
                    });
                }
                6 => Some(Ok(Message::REQUEST)),
                7 => {
                    return Some(match Block::from(bytes) {
                        Ok(block) => Ok(Message::PIECE(block)),
                        Err(e) => Err(e),
                    });
                }
                8 => Some(Ok(Message::CANCEL)),
                9 => Some(Ok(Message::PORT)),
                20 => Some(Ok(Message::EXTENDED(Extended::from(bytes)))),
                _ => None,
            }
        }
    }
}

/// A wrapper around ReadHalf of the TCPStream
///
/// TODO : Study about tokio_codec and try to use it here
///
/// When some data comes that works on certain protocol on top of TCP, like my Bittorent Protocol
/// that i implemented here, data may not arrive in single packet, and if we use "read_buf" or
/// "read" method on the "ReadHalf" then we might read an incomplete data that needs some more data
/// from TCP packets and this can cause lot of issues.
///
/// In order to make sure we are reding a complete data, what we will do is whenever some
/// response comes we'll see the length of the data using "len" and compare it with the length
/// mentioned in the "length_prefix" of the data:w
///
pub struct TCPReceiver<'a> {
    read_half: ReadHalf<'a>,
}
impl<'a> TCPReceiver<'a> {
    /// Creates a new TCPReceiver instance
    fn new(read_half: ReadHalf<'a>) -> Self {
        Self { read_half }
    }

    /// Reads on the TCP socket until a Message is found
    /// NOTE : On error, drop the connection!
    async fn getMessage(&mut self) -> Result<Vec<Message>> {
        // It's the max amount of data we'll ever receive, which is the max size of block we're
        // ever gonna request
        const MAX_BUFFER_CAPACITY: usize = 17000;

        let mut messages: Vec<Message> = Vec::new();
        let mut buf = BytesMut::with_capacity(MAX_BUFFER_CAPACITY);
        'main: loop {
            if let Ok(size) = self.read_half.read_buf(&mut buf).await {
                match size {
                    // If the returned "size" is 0, then its EOF, which means the connection was closed
                    0 => {
                        return Err("EOF".into());
                    }
                    _ => {
                        // In the Bittorent Protocol, the first message we send is a HANDSHAKE message
                        // after connecting to a peer. We expect a HANDSHAKE and BITFIELD, EXTENDED or
                        // HAVE immediately followed to that HANDSHAKE response in a different TCP packet
                        // to be sent by the peer to us as a response. Some peers send them as different packet
                        // but some peers send them on the same packet that they sent the HANDSHAKE
                        // response, so in order to extract all these messages if they exist we try to find multiple
                        // messages on the buffer
                        //
                        // Loops until it finds all messages to be extracted from buffer, or if it
                        // finds an error, then it means it needs to read more tcp packet and try
                        // again until "None" is emitted
                        while let Some(Ok(msg)) = messageHandler(&mut buf, self).await {
                            messages.push(msg);
                        }
                        if messageHandler(&mut buf, self).await.is_none() {
                            break 'main;
                        }
                    }
                }
            } else {
                return Err("Some Error Occured".into());
            }
        }
        Ok(messages)
    }
}

/// A wrapper around write half of the TCPStream :
pub struct TCPSender<'a> {
    write_half: WriteHalf<'a>,
    details: __Details,
    receiver: UnboundedReceiver<Vec<Message>>,
}

impl<'a> TCPSender<'a> {
    /// Creates a new TCPSender instance
    fn new(write_half: WriteHalf<'a>, details: __Details, receiver: UnboundedReceiver<Vec<Message>>) -> Self {
        Self { write_half, details, receiver }
    }

    /// Creates a HANDSHAKE message and sends the Handshake Message to the peer
    /// and returns the responses of that Handshake Message
    ///
    /// NOTE : It drops the connection as soon as it sees CHOKE message as a response
    /// of the HANDSHAKE message
    pub async fn sendHandshakeMessage(&mut self) -> Result<Vec<Message>> {
        const HANDSHAKE_RESPONSE_WAIT_TIME: u64 = 2;

        // Creates a HANDSHAKE Message
        let mut handshake_msg = Handshake::default();
        let lock_details = self.details.lock().await;
        let info_hash = lock_details.info_hash.as_ref().unwrap().clone();
        handshake_msg.set_info_hash(info_hash);
        drop(lock_details);

        // Writes the HANDSHAKE message on the TCPStream
        self.write_half.write_all(&handshake_msg.getBytesMut()).await;

        // Waits for all the messages that peer is gonna send as response to the HANDSHAKE message we sent
        let mut messages = Vec::new();
        if let Some(mut msgs) = self.receiver.recv().await {
            messages.append(&mut msgs);
            // Store all responses sent after 2 seconds of receiving HANDSHAKE response, its usually BITFIELD/HAVE/EXTENDED
            timeout(Duration::from_secs(HANDSHAKE_RESPONSE_WAIT_TIME), async {
                loop {
                    if let Some(mut _msgs) = self.receiver.recv().await {
                        messages.append(&mut _msgs);
                    }
                }
            })
            .await;
        }

        // If the peer sends CHOKE, then we'll disconnect from that peer
        if messages.contains(&Message::CHOKE) {
            self.write_half.shutdown();
        }
        Ok(messages)
    }

    /// Writes INTERESTED message on the TCPStream
    pub async fn sendInterestedMessage(&mut self) -> Option<Vec<Message>> {
        self.write_half.write_all(&Interested::build_message()).await;

        self.receiver.recv().await
    }

    /// Writes UNCHOKE message on the TCPStream
    pub async fn sendUnchokeMessage(&mut self) {
        self.write_half.write_all(&Unchoke::build_message()).await;
    }
}
