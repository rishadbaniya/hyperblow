#![allow(non_camel_case_types)]
use super::start::__Details;
use crate::Result;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, Bytes, BytesMut};
use crossterm::event::read;
use futures::join;
use sha1::digest::generic_array::typenum::Len;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::fs::write;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

//
// Protocol Implemented From :  https://wiki.theory.org/indx.php/BitTorrentSpecification#Handshakee
// In version 1.0 of the BitTorrent protocol, pstrlen = 19, and pstr = "BitTorrent protocol".

// 0. pstrlen => Single byte value which is length of "pstr", i.e u8 (Value = 19)
// 1. pstr => String identifier of the protocol (Value = "BitTorrent protocol" )
// 2. reserved => 8 reserved bytes. Current implentation uses all zeroes
// 3.
struct Handshake {
    pub pstrlen: u8,
    pub pstr: Vec<u8>,
    pub reserved: Vec<u8>,
    pub info_hash: Option<Vec<u8>>,
    pub peer_id: Vec<u8>,
}

impl Handshake {
    fn empty() -> Self {
        let pstrlen: u8 = 19;
        let pstr: Vec<u8> = b"BitTorrent protocol".map(|v| v).into_iter().collect();
        let reserved = vec![0; 8];
        let peer_id = b"-HYBLOW-110011001100".map(|v| v).into_iter().collect();
        Self {
            pstrlen,
            pstr,
            reserved,
            info_hash: None,
            peer_id,
        }
    }

    fn from(v: BytesMut) -> Self {
        let v: Vec<u8> = v.into_iter().collect();
        let bytes_peer_id: Vec<u8> = v[49..].iter().map(|v| *v).collect();

        let pstrlen = v[0];
        let pstr: Vec<u8> = v[1..=19].iter().map(|v| *v).collect();
        let reserved: Vec<u8> = v[20..=27].iter().map(|v| *v).collect();
        let info_hash: Option<Vec<u8>> = Some(v[28..=48].iter().map(|v| *v).collect());
        let peer_id: Vec<u8> = v[49..].iter().map(|v| *v).collect();

        Self {
            pstrlen,
            pstr,
            reserved,
            info_hash,
            peer_id,
        }
    }

    fn getBytesMut(&self) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_u8(self.pstrlen);
        buf.put_slice(self.pstr.as_slice());
        buf.put_slice(self.reserved.as_slice());
        buf.put_slice(self.info_hash.as_ref().unwrap().as_slice());
        buf.put_slice(self.peer_id.as_slice());
        buf
    }

    fn set_info_hash(&mut self, v: Vec<u8>) {
        self.info_hash = Some(v);
    }
}

//
// PEER REQUEST (TCP)
//
// Objective : Connect to Peers and download pieces(blocks)
// First of all, we make a TCP connection with the "peer", after making TCP connection with the
// peer
pub async fn peer_request(socket_adr: SocketAddr, details: __Details) {
    const CONNECTION_TIMEOUT: u64 = 15;

    loop {
        // Attempts to make a TCP connection with the peer until 15 seconds has passed
        match timeout(Duration::from_secs(CONNECTION_TIMEOUT), TcpStream::connect(socket_adr)).await {
            // Means TCP connection was established
            Ok(v) => match v {
                Ok(mut stream) => {
                    // Split the TCP stream into read and write half
                    let (mut read_half, mut write_half) = stream.split();
                    let (sender, mut receiver) = mpsc::channel::<Message>(2000);
                    // Continuosly read on the TCP stream
                    let read = async move {
                        'read_loop: loop {
                            let mut buf = BytesMut::with_capacity(512);
                            match read_half.read_buf(&mut buf).await {
                                Ok(v) => {
                                    if v == 0 {
                                        break 'read_loop;
                                    }
                                    let value = messageHandler(&buf);
                                    sender.send(value).await.unwrap();
                                }
                                Err(e) => println!("{:?}", e),
                            }
                        }
                    };

                    // TCP stream is bidirectional, so once we have sent a Handshake message, we
                    // will until some message has arrived on the Stream's socket, if the message
                    // has arrived then we read it and deserialize it to certain Message Type, at
                    // first we're gonna expect it to be a Handshake Message.
                    //
                    //
                    //
                    let write_details = details.clone();
                    let write = async move {
                        let mut received_messages: Vec<Message> = Vec::new();
                        // Build data for Handshake Request
                        let mut handshake_request = Handshake::empty();
                        handshake_request.set_info_hash(write_details.lock().await.info_hash.as_ref().unwrap().clone());
                        let handshake_request = handshake_request.getBytesMut();

                        write_half.write_all(&handshake_request).await.unwrap();

                        'main: loop {
                            //
                            if let Some(v) = receiver.recv().await {
                                received_messages.push(v);
                                //
                                // If it's the first phase i.e peer has sent a Handshake message
                                if v == Message::HANDSHAKE {
                                    // Wait maximum of 5 seconds for the next message
                                    match timeout(Duration::from_secs(5), receiver.recv()).await {
                                        Ok(v) => {
                                            received_messages.push(v.unwrap());
                                            match timeout(Duration::from_secs(5), receiver.recv()).await {
                                                Ok(v) => {
                                                    received_messages.push(v.unwrap());
                                                    if received_messages.contains(&Message::CHOKE) {
                                                        write_half.shutdown().await.unwrap();
                                                        break 'main;
                                                    }
                                                    write_half.write_all(&InterestedMessage::getBytes()).await.unwrap();

                                                    println!("{:?}", received_messages);
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ => {}
                                    }
                                } else if received_messages[0] == Message::HANDSHAKE {
                                    if v == Message::UNCHOKE {
                                        write_half.write_all(&UnchokeMessage::getBytes()).await.unwrap();
                                        println!("{:?}", v);
                                    } else if v == Message::CHOKE {
                                        write_half.shutdown().await.unwrap();
                                    } else {
                                    }
                                }
                            }
                        }
                    };

                    join!(write, read);

                    // Send Handshake Request through the connected TCP socket
                    //if write_half.write_all(&handshake_request).await.is_ok() {
                    //let mut buf = BytesMut::with_capacity(1024);
                    // Waits for some data to arrive on the TCP socket
                    //     read_half.readable().await.unwrap();
                    //     // When the data is availaible, we read it into the buffer
                    //     let s = read_half.read_buf(&mut buf).await.unwrap();
                    //     match messageHandler(&buf) {
                    //         MessageType::HANDSHAKE => {
                    //             let interested_request = InterestedMessage::getBytesMut();
                    //             if write_half.write_all(&&interested_request).await.is_ok() {
                    //                 read_half.readable().await.unwrap();
                    //                 let mut buf = BytesMut::with_capacity(1024);
                    //                 let s = read_half.read_buf(&mut buf).await.unwrap();
                    //                 println!("After i sent interested i got {:?} of length {}", messageHandler(&buf), buf.len());
                    //             }
                    //         }
                    //         _ => {}
                    //     }
                    //}
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

// Messages send to the peer and recieved form the peer takes the following forms
//
// All possible messages are specified by BitTorrent Specifications at :
// https://wiki.theory.org/index.php/BitTorrentSpecification#Messages
#[derive(PartialEq, Debug, Clone, Copy)]
enum Message {
    HANDSHAKE,
    KEEP_ALIVE,
    CHOKE,
    UNCHOKE,
    INTERESTED,
    NOT_INTERESTED,
    HAVE,
    BITFIELD,
    REQUEST,
    PIECE,
    CANCEL,
    PORT,
    INCONSISTENT,
    UNKNOWN,
}

// Takes a reference to the message sent by the peer and finds out what kind of message was sent
// by the peer
//
// All possible messages are specified by BitTorrent Specifications at :
// https://wiki.theory.org/index.php/BitTorrentSpecification#Messages
//
// NOTE : A peer can also choose to send a Handshake Message immediately followed by a Bitfield
// message in the same packet
//
fn messageType(message: &BytesMut) -> Message {
    if message.len() < 4 {
        Message::UNKNOWN
    } else if message.len() == 4 {
        Message::KEEP_ALIVE
    } else if message.len() == 68 {
        Message::HANDSHAKE
    } else {
        let message_id = *message.get(4).unwrap();
        match message_id {
            0 => Message::CHOKE,
            1 => Message::UNCHOKE,
            2 => Message::INTERESTED,
            3 => Message::NOT_INTERESTED,
            4 => Message::HAVE,
            5 => Message::BITFIELD,
            6 => Message::REQUEST,
            7 => Message::PIECE,
            8 => Message::CANCEL,
            9 => Message::PORT,
            _ => Message::INCONSISTENT,
        }
    }
}

// Struct to build a Request Message and deserialize peer's Request Message
//
// length_prefix => 13u32 (Total length of the payload that follows the initial 4 bytes)
// id => u8 (id of the message)
// index => (index of the piece)
// begin => (index of the beginning byte)
// length => (length of the piece from beginning offset)
//
struct RequestMessage {
    length_prefix: u32,
    id: u8,
    index: u32,
    begin: u32,
    length: u32,
}

impl RequestMessage {
    pub fn new(index: u32, begin: u32, length: u32) -> Self {
        //let mut buf = BytesMut::new();
        Self {
            length_prefix: 13,
            id: 6,
            index,
            begin,
            length,
        }
    }

    pub fn getBytesMut(&self) -> BytesMut {
        let mut bytes_mut: BytesMut = BytesMut::new();
        bytes_mut.put_u32(self.length_prefix);
        bytes_mut.put_u8(self.id);
        bytes_mut.put_u32(self.index);
        bytes_mut.put_u32(self.begin);
        bytes_mut.put_u32(self.length);
        bytes_mut
    }
}

// Interested message
struct InterestedMessage;

impl InterestedMessage {
    fn getBytes() -> BytesMut {
        let mut bytes_mut = BytesMut::new();
        bytes_mut.put_u32(1);
        bytes_mut.put_u8(2);
        bytes_mut
    }
}

// Unchoke Message
struct UnchokeMessage;

impl UnchokeMessage {
    pub fn getBytes() -> BytesMut {
        let mut bytes_mut = BytesMut::new();
        bytes_mut.put_u32(1);
        bytes_mut.put_u8(1);
        bytes_mut
    }
}

fn messageHandler(message: &BytesMut) -> Message {
    messageType(&message)
}
