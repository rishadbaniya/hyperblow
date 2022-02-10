#![allow(non_camel_case_types)]
use super::start::__Details;
use crate::Result;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use sha1::digest::generic_array::typenum::Len;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
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

                    // Build data for Handshake Request
                    let mut handshake_request = Handshake::empty();
                    handshake_request.set_info_hash(details.lock().await.info_hash.as_ref().unwrap().clone());
                    let handshake_request = handshake_request.getBytesMut();

                    // Send Handshake Request through the connected TCP socket
                    if write_half.write_all(&handshake_request).await.is_ok() {
                        let mut buf = BytesMut::with_capacity(1024);
                        // Waits for some data to arrive on the TCP socket
                        read_half.readable().await.unwrap();
                        // When the data is availaible, we read it into the buffer
                        let s = read_half.read_buf(&mut buf).await.unwrap();
                        messageHandler(&buf);
                    }
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
#[derive(PartialEq, Debug)]
enum MessageType {
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
    UNKNOWN,
}

// Takes a reference to the message sent by the peer and finds out what kind of message was sent
// by the peer
//
// All possible messages are specified by BitTorrent Specifications at :
// https://wiki.theory.org/index.php/BitTorrentSpecification#Messages
fn messageType(message: &BytesMut) -> MessageType {
    if message.len() == 0 {
        MessageType::KEEP_ALIVE
    } else if message.len() == 68 {
        MessageType::HANDSHAKE
    } else {
        let message_id = *message.get(4).unwrap();
        match message_id {
            0 => MessageType::CHOKE,
            1 => MessageType::UNCHOKE,
            2 => MessageType::INTERESTED,
            3 => MessageType::NOT_INTERESTED,
            4 => MessageType::HAVE,
            5 => MessageType::BITFIELD,
            6 => MessageType::REQUEST,
            7 => MessageType::PIECE,
            8 => MessageType::CANCEL,
            9 => MessageType::PORT,
            _ => {
                println!("{:?}", message);
                MessageType::UNKNOWN
            }
        }
    }
}

// Struct to build a Request Message and deserialize peer's Request Message
//
// length_prefix => 13u32 (Total length of the payload)
// id => u8 (id of the message)
// index => (index of the piece)
// begin => (index of the beginning byte)
// begin => (length of the piece from beginning offset)
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

fn messageHandler(message: &BytesMut) {
    let message_type = messageType(&message);
    println!("{:?}", message_type);
}
