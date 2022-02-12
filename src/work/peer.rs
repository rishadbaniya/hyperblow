#![allow(non_camel_case_types)]
#![allow(unused_must_use)]
#![allow(dead_code)]

use super::message::{Bitfield, Extended, Handshake, Have};
use super::start::{self, __Details};
use crate::work::message::{Interested, Request, Unchoke};
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use futures::{select, FutureExt};
use sha1::digest::generic_array::sequence::Split;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

// PEER REQUEST (TCP) :
//
// OBJECTIVE : Connect to Peers and download pieces chunk by chunk
// We do it by making a TCP connection with the "peer"
//
// NOTE : A torrent contains multiple pieces and pieces contain multiple chunks, each chunk should
// not be greater than 16 Kb, i.e we should not request data greater than 16 Kb in request message
//
const CONNECTION_TIMEOUT: u64 = 60;
const MAX_CHUNK_LENGTH: u32 = 16384;
const CONNECTION_FAILED_TRY_AGAIN_AFTER: u64 = 60;
const MAX_TCP_WINDOW_SIZE: u32 = 65_535;

pub async fn peer_request(socket_adr: SocketAddr, details: __Details) {
    let PIECE_LENGTH = details.lock().await.piece_length.unwrap() as u32;

    loop {
        // Tries to make a TCP connection with the peer until CONNECTION_TIMEOUT has passed
        match timeout(Duration::from_secs(CONNECTION_TIMEOUT), TcpStream::connect(socket_adr)).await {
            // When TCP connection is established
            Ok(v) => match v {
                Ok(mut stream) => {
                    // Split the TCP stream into read and write half
                    let (mut read_half, mut write_half) = stream.split();

                    // Channel to communicate between read and write half :
                    let (sender, mut receiver) = mpsc::channel::<BytesMut>(MAX_TCP_WINDOW_SIZE as usize);

                    // READ HALF :
                    // Continuosly reads on the TCP stream until EOF
                    let read = async move {
                        'main: loop {
                            let mut buf = BytesMut::with_capacity(MAX_TCP_WINDOW_SIZE as usize);
                            match read_half.read_buf(&mut buf).await {
                                Ok(v) => {
                                    if v == 0 {
                                        break 'main;
                                    }
                                    sender.send(buf).await.unwrap();
                                }
                                Err(e) => println!("{:?}", e),
                            }
                        }
                    };

                    // WRITE HALF :
                    //
                    // NOTE :According to my knowledge, TCP is not req-res like most protocols built on top of TCP, HTTP is req-res not because its
                    // built on top TCP
                    //
                    // TCP is bidirectional, so once we have sent a Handshake message, we
                    // can wait until some message has arrived on the Socket, if the messages
                    // has arrived then we read it and deserialize it to certain Message Types
                    let write_details = details.clone();
                    let write = async move {
                        let mut messages: Vec<Message> = Vec::new();
                        // Writes "Handshake message" on the TCP stream
                        write_half.write_all(&createHandshakeMsg(write_details).await).await.unwrap();

                        // NOTE : This block must run once
                        //
                        // First phase i.e peer has sent responses for the "Handshake" message we sent
                        // It gets all the message that peer has sent as a response and
                        // deserializes it, it even handles incosistency among message, like
                        // mulitple messages sent by peer as one single and multiple messages sent
                        // in different packet
                        if let Some(msg) = receiver.recv().await {
                            // If messages is empty, then it means we were waiting for some message
                            // to come after a Handshake request was sent
                            if messages.is_empty() {
                                // We'll push all the bytes sent to us by the peer as a response of the "Handshake" message we sent into one big chunk
                                // i.e "response_from_handshake" and then deserialize all messages out of this big chunk
                                let mut response_from_handshake = BytesMut::new();
                                response_from_handshake.put_slice(&msg);
                                timeout(Duration::from_secs(2), async {
                                    loop {
                                        if let Some(v) = receiver.recv().await {
                                            response_from_handshake.put_slice(&v);
                                        }
                                    }
                                })
                                .await;
                                messages.append(&mut handshake_responses(&mut response_from_handshake));
                            }
                        }

                        // On Choke, shutdown the TCP stream and stop progression of the future
                        if messages.contains(&Message::CHOKE) {
                            write_half.shutdown();
                            return;
                        }

                        // Send interested message to the peers and unchoke expect them to send unchoke message
                        write_half.write_all(&Interested::build_message()).await.unwrap();

                        if let Some(mut msg) = receiver.recv().await {
                            messages.push(messageHandler(&mut msg).unwrap());
                        }

                        // On Choke, shutdown the TCP stream and stop progression of the future
                        if messages.contains(&Message::CHOKE) {
                            write_half.shutdown();
                            return;
                        }
                        if messages.contains(&Message::UNCHOKE) {
                            write_half.write_all(&Unchoke::build_message()).await.unwrap();
                        }

                        let pieces = Pieces::new(&mut messages);

                        //println!("{:?}", pieces.have);
                        let mut chunks: Vec<Chunk> = Vec::new();
                        let current_piece = pieces.have[0];
                        let mut begin_offset: u32 = 0;
                        let mut piece = pieces.have[0];
                        loop {
                            if !begin_offset >= PIECE_LENGTH {
                                write_half
                                    .write_all(&Request::build_message(pieces.have[0], begin_offset, MAX_CHUNK_LENGTH))
                                    .await
                                    .unwrap();
                                if let Some(mut msg) = receiver.recv().await {
                                    let chunk = Chunk::from(&mut msg).unwrap();
                                    begin_offset = begin_offset + chunk.chunk_length - 1;
                                    println!("FULL PIECE SIZE : {}", PIECE_LENGTH);
                                    println!("DOWNLOADING {}", pieces.have[0]);
                                    println!("Total Chunks i had {} and byte_end_index {}", chunks.len(), begin_offset);
                                    chunks.push(chunk);
                                }
                            }
                        }
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
                sleep(Duration::from_secs(240)).await
            }
        }
        sleep(Duration::from_secs(100)).await;
    }
}

struct Pieces {
    have: Vec<u32>,
}

impl Pieces {
    fn new(v: &mut Vec<Message>) -> Self {
        let mut have: Vec<u32> = Vec::new();
        *v = v
            .iter()
            .filter(|f| {
                match f {
                    Message::BITFIELD(_bitfield) => {
                        have.append(&mut _bitfield.have.clone());
                    }
                    Message::HAVE(_have) => {
                        have.push(_have.piece_index);
                    }
                    _ => {
                        return true;
                    }
                }
                return false;
            })
            .map(|v| v.clone())
            .collect();
        Self { have }
    }
}

/// Creates a Handshake Message and gives us a buffer containing
/// Handshake Message that we can send to the "peer"
async fn createHandshakeMsg(details: __Details) -> BytesMut {
    let mut handshake_msg = Handshake::empty();
    let lock_details = details.lock().await;
    let info_hash = lock_details.info_hash.as_ref().unwrap().clone();
    handshake_msg.set_info_hash(info_hash);
    handshake_msg.getBytesMut()
}

fn handshake_responses(bytes: &mut BytesMut) -> Vec<Message> {
    let mut messages: Vec<Message> = Vec::new();
    while let Some(msg) = messageHandler(bytes) {
        messages.push(msg);
    }
    messages
}

// NOTE : Digests bytes acoording to the message found
// This function removes bytes from buffer that is provided after it founds a message
fn messageHandler(bytes: &mut BytesMut) -> Option<Message> {
    if bytes.len() == 0 {
        // If the buffer is empty then it means there is no message
        None
    } else if bytes.len() == 4 {
        // TODO : Check if the length is (0_u32) as well
        Some(Message::KEEP_ALIVE)
    } else {
        // If it's a HANDSHAKE message, then the first message is pstr_len, whose value is 19
        // TODO : Check if pstr = "BitTorrent protocol" as well
        let pstr_len = bytes[0];

        if pstr_len == 19u8 {
            let handshake_msg = Handshake::from(&bytes.split_to(68));
            Some(Message::HANDSHAKE(handshake_msg))
        } else {
            let mut message_id = 100;
            if let Some(v) = bytes.get(4) {
                message_id = *v;
            }
            match message_id {
                0 => {
                    bytes.split_to(5);
                    Some(Message::CHOKE)
                }
                1 => {
                    bytes.split_to(5);
                    Some(Message::UNCHOKE)
                }
                2 => {
                    bytes.split_to(5);
                    Some(Message::INTERESTED)
                }
                3 => {
                    bytes.split_to(5);
                    Some(Message::NOT_INTERESTED)
                }
                4 => Some(Message::HAVE(Have::from(bytes))),
                5 => Some(Message::BITFIELD(Bitfield::from(bytes))),
                6 => Some(Message::REQUEST),
                7 => Some(Message::PIECE),
                8 => Some(Message::CANCEL),
                9 => Some(Message::PORT),
                20 => Some(Message::EXTENDED(Extended::from(bytes))),
                _ => None,
            }
        }
    }
}

//
// Messages sent to the peer and recieved form the peer takes the following forms
//
// All possible messages are specified by BitTorrent Specifications at :
//
// https://wiki.theory.org/index.php/BitTorrentSpecification#Messages
// https://www.bittorrent.org/beps/bep_0010.html
//
#[derive(PartialEq, Debug, Clone)]
enum Message {
    HANDSHAKE(Handshake),
    BITFIELD(Bitfield),
    EXTENDED(Extended),
    HAVE(Have),
    KEEP_ALIVE,
    CHOKE,
    UNCHOKE,
    INTERESTED,
    NOT_INTERESTED,
    REQUEST,
    PIECE,
    CANCEL,
    PORT,
}

/// Stores block's data and its metadata sent by a peer
#[derive(Debug, Clone)]
struct Chunk {
    len: u32,
    id: u8,
    piece_index: u32,
    byte_start_index: u32,
    byte_end_index: u32,
    chunk_length: u32,
    chunk: BytesMut,
}

impl Chunk {
    fn from(bytes: &mut BytesMut) -> Option<Self> {
        let len: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[0..=3]).unwrap();
        let id: u8 = *bytes.get(4).unwrap();
        let piece_index: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[5..=8]).unwrap();
        let byte_start_index: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[9..=12]).unwrap();
        bytes.split_to(13);
        let chunk = bytes.clone();
        let byte_end_index = (chunk.len() - 1) as u32;

        Some(Self {
            len,
            id,
            piece_index,
            byte_start_index,
            byte_end_index,
            chunk_length: chunk.len() as u32,
            chunk,
        })
    }
}
