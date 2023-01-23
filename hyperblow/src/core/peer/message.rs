// NOTE : This file contains all the structs and methods related to
// Bittorent Message
//
// All the messages and their specified protocol is taken from :
// https://wiki.theory.org/index.php/BitTorrentSpecification#Messages
// https://www.bittorrent.org/beps/bep_0010.html
//
// Initially we as a peer start as :
//
// NOT_INTERESTED and
// CHOKE

#![allow(unused_must_use)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use super::Block;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use serde_derive::{Deserialize, Serialize};

/// Messages sent to the peer and recieved form the peer takes the following forms
///
/// All possible messages are specified by BitTorrent Specifications at :
///
#[derive(PartialEq, Debug, Clone)]
pub enum Message {
    HANDSHAKE(Handshake),
    BITFIELD(Bitfield),
    EXTENDED(Extended),
    HAVE(Have),
    PIECE(Block),
    KEEP_ALIVE,
    CHOKE,
    UNCHOKE,
    INTERESTED,
    NOT_INTERESTED,
    REQUEST,
    CANCEL,
    PORT,
}

/// INTERESTED message
///
/// INTERESTED Message is sent after HANDSHAKE message has been
/// exchanged between the peers, its used to tell the peer "hey i'm interested in exchanging files with you",
/// and the peer can choose choke or unchoke us by sending us CHOKE or UNCHOKE message,
/// we can then choose to CHOKE and UNCHOKE the user after by looking at the response of this INTERESTED message
///
/// NOTE : Sending an INTERESTED message doesn't guarantee that the peer would send UNCHOKE
pub struct Interested;

impl Interested {
    pub fn build_message() -> BytesMut {
        let mut bytes_mut = BytesMut::new();
        bytes_mut.put_u32(1);
        bytes_mut.put_u8(2);
        bytes_mut
    }
}

/// Unchoke Message
///
///
pub struct Unchoke;

impl Unchoke {
    pub fn build_message() -> BytesMut {
        let mut bytes_mut = BytesMut::new();
        bytes_mut.put_u32(1);
        bytes_mut.put_u8(1);
        bytes_mut
    }
}

/// Request Message
///
/// length_prefix => 13u32 (Total length of the payload that follows the initial 4 bytes)
/// id => u8 (id of the message)
/// index => (index of the piece)
/// begin => (index of the beginning byte)
/// length => (length of the piece from beginning offset)
pub struct Request {
    length_prefix: u32,
    id: u8,
    index: u32,
    begin: u32,
    length: u32,
}

impl Request {
    // TODO : Remove all parameters and make it BytesMut
    pub fn from(index: u32, begin: u32, length: u32) -> Self {
        //let mut buf = BytesMut::new();
        Self {
            length_prefix: 13,
            id: 6,
            index,
            begin,
            length,
        }
    }

    pub fn build_message(index: u32, begin: u32, length: u32) -> BytesMut {
        let length_prefix = 13;
        let id = 6;
        let mut bytes_mut: BytesMut = BytesMut::new();
        bytes_mut.put_u32(length_prefix);
        bytes_mut.put_u8(id);
        bytes_mut.put_u32(index);
        bytes_mut.put_u32(begin);
        bytes_mut.put_u32(length);
        bytes_mut
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bitfield {
    pub have: Vec<u32>,
    pub not_have: Vec<u32>,
}

impl Bitfield {
    pub fn from(bytes: &mut BytesMut) -> crate::Result<Self> {
        let mut have: Vec<u32> = Vec::new();
        let mut not_have: Vec<u32> = Vec::new();

        let length_prefix: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[0..=3]).unwrap();
        let bitfield_total_length = (length_prefix + 4) as usize;

        // If the bytes size is less than bitfield total length then it means it needs more tcp
        // packet to combine and create its full data
        if bytes.len() < bitfield_total_length {
            return Err("WAIT FOR MORE TCP SEGMENT".into());
        } else {
            let bitfield_payload = bytes.split_to(bitfield_total_length);
            for i in 0..bitfield_payload.len() - 1 {
                match bitfield_payload[i] {
                    0 => not_have.push(i as u32),
                    1 => have.push(i as u32),
                    _ => {}
                }
            }
        }

        Ok(Self { have, not_have })
    }
}

/// Extended Message :
///
/// A message type for those who implement the Extension Protocol
/// from - http://www.bittorrent.org/beps/bep_0010.html
///
/// Structure :
/// length_prefix => u32 (No of bytes for the entire message) : Offset [0,3]
/// message_id => u8 (value = 20) (id of the message) : Offset : [4]
/// extension_message_id => u8 (id of the message) : Offset : [5]
#[derive(Debug, Clone, PartialEq)]
pub struct Extended {
    length_prefix: u32,
    message_id: u8,
    extension_message_id: u8,
    payload: ExtendedPayload,
}

impl Extended {
    pub fn from(bytes: &mut BytesMut) -> Self {
        let length_prefix: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[0..=3]).unwrap();
        let message_id: u8 = bytes[4];
        let extension_message_id: u8 = bytes[5];
        bytes.split_to(6);

        let payload_length = (length_prefix - 2) as usize;
        let payload = bytes.split_to(payload_length);
        let payload: ExtendedPayload = serde_bencode::de::from_bytes(&payload).unwrap();

        Extended {
            length_prefix,
            message_id,
            extension_message_id,
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct ExtendedPayload {
    pub v: Option<String>,
}

/// HAVE Message :
/// TODO : Add info about HAVE message
#[derive(Debug, PartialEq, Clone)]
pub struct Have {
    pub piece_index: u32,
}

impl Have {
    pub fn from(bytes: &mut BytesMut) -> Self {
        let piece_index: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[5..=8]).unwrap();
        bytes.split_to(9);
        Self { piece_index }
    }
}

/// HANDSHAKE Message :
///
/// It's the first message to be exchanged by us (the initiator of the connection), with the peer.
/// A Handshake message has fixed 68 byte length. The peer also sends HANDSHAKE as the first
/// message to us.
///
/// NOTE : If any peer sends message other than HANDSHAKE as the first message when we send
/// them HANDSHAKE message, then we must terminate the Connection.
///
/// Structure :
///
/// pstrlen => length of pstr (u8) (value = 19)
/// pstr => b"BitTorrent Protocol"
/// reserved => 8 reserved bytes, if no extension is used then its usually all 0's, use to define Extensions Used
/// info_hash => The info hash of the torrent (20 bytes)
/// peer_id => Peer id of the peer (20 bytes)
///
///
/// For More : https://wiki.theory.org/indx.php/BitTorrentSpecification#Handshakee
///
#[derive(Debug, Clone, PartialEq)]
pub struct Handshake {
    pub pstrlen: u8,
    pub pstr: Vec<u8>,
    pub reserved: Vec<u8>,
    pub info_hash: Option<Vec<u8>>,
    pub peer_id: Vec<u8>,
}

impl Default for Handshake {
    fn default() -> Self {
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
}

impl Handshake {
    pub fn from(v: &mut BytesMut) -> Self {
        let pstrlen = v.split_to(1).to_vec()[0];
        let pstr: Vec<u8> = v.split_to(19).to_vec();
        let reserved: Vec<u8> = v.split_to(8).to_vec();
        let info_hash: Option<Vec<u8>> = Some(v.split_to(20).to_vec());
        let peer_id: Vec<u8> = v.split_to(20).to_vec();

        Self {
            pstrlen,
            pstr,
            reserved,
            info_hash,
            peer_id,
        }
    }

    pub fn getBytesMut(&self) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_u8(self.pstrlen);
        buf.put_slice(self.pstr.as_slice());
        buf.put_slice(self.reserved.as_slice());
        buf.put_slice(self.info_hash.as_ref().unwrap().as_slice());
        buf.put_slice(self.peer_id.as_slice());
        buf
    }

    pub fn set_info_hash(&mut self, v: Vec<u8>) {
        self.info_hash = Some(v);
    }
}
