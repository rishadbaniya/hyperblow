#![allow(non_camel_case_types)]
#![allow(unused_must_use)]

use super::{start::__Details, Bitfield, Block, Extended, Handshake, Have, Interested, Message, Unchoke};
use crate::work::Request;
use crate::Result;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use futures::{select, FutureExt};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{
    tcp::{ReadHalf, WriteHalf},
    TcpStream,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::time::{sleep, timeout};

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
        match timeout(Duration::from_secs(CONNECTION_TIMEOUT), TcpStream::connect(socket_adr)).await {
            Ok(v) => match v {
                Ok(mut stream) => {
                    // Splits the stream and also creates channel to communicate between those two
                    // halves
                    let (read_half, write_half) = stream.split();
                    let (sender, receiver) = unbounded_channel::<Vec<Message>>();

                    // READ HALF :
                    // Continuosly reads for message on the TCP stream until EOF
                    let read = async move {
                        let mut tcp_receiver = TCPReceiver::new(read_half);
                        loop {
                            if let Ok(msgs) = tcp_receiver.getMessage().await {
                                sender.send(msgs);
                            } else {
                                break;
                            }
                        }
                    };

                    // WRITE HALF :
                    let _details = details.clone();
                    let write = async move {
                        let mut messages: Vec<Message> = Vec::new();
                        let mut tcp_sender = TCPSender::new(write_half, _details, receiver);

                        // Sends HANDSHAKE message
                        let mut handshake_response = tcp_sender.sendHandshakeMessage().await.unwrap();
                        messages.append(&mut handshake_response);

                        // Sends INTERESTED message
                        let mut interested_response = tcp_sender.sendInterestedMessage().await.unwrap();
                        messages.append(&mut interested_response);

                        // Sends UNCHOKE message
                        tcp_sender.sendUnchokeMessage();

                        let peer_details = PeerDetails::from(&mut messages);

                        if !peer_details.pieces_have.is_empty() {
                            let piece_index = peer_details.pieces_have[0];
                            let mut byte_offset: u32 = 0;
                            if PIECE_LENGTH != byte_offset + 1 {
                                tcp_sender
                                    .write_half
                                    .write_all(&Request::build_message(piece_index, byte_offset, MAX_BUFFER_CAPACITY))
                                    .await;

                                if let Some(v) = tcp_sender.receiver.recv().await {
                                    println!("{:?}", v.len());
                                }
                            }
                        }

                        println!("{:?}", messages);
                    };

                    // End both the future as soon as one gets completed
                    select! {
                        () = read.fuse() => (),
                        () = write.fuse() => ()
                    };
                }
                Err(_) => {
                    // Connection Refused or Something related with Socket address
                    sleep(Duration::from_secs(240)).await
                }
            },
            Err(_) => {
                // Timeout Error
                sleep(Duration::from_secs(CONNECTION_FAILED_TRY_AGAIN_AFTER)).await;
            }
        }
        sleep(Duration::from_secs(2000)).await;
    }
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
struct TCPReceiver<'a> {
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
        const MAX_BUFFER_CAPACITY: usize = 16384;

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
struct TCPSender<'a> {
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
