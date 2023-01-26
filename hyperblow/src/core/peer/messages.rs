/*
 *
 *NOTE : This file contains all the structs and methods related to
 *Bittorent Message
 *
 *All the messages and their specified protocol is taken from :
 *https://wiki.theory.org/index.php/BitTorrentSpecification#Messages
 *https://www.bittorrent.org/beps/bep_0010.html
 *Initially we as a peer start as :
 *
 * NOT_INTERESTED and
 * CHOKE
 *
 *
 *
 *
 *
 *
 */

#![allow(unused_must_use)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

//use core::slice::SlicePattern;
//use std::sync::Arc;
//
//use byteorder::{BigEndian, ReadBytesExt};
//use bytes::{BufMut, BytesMut};
use crate::core::state::State;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use std::sync::Arc;
//use serde_derive::{Deserialize, Serialize};

/// Messages sent to the peer and recieved form the peer takes the following forms
///
/// All possible messages are specified by BitTorrent Specifications at :
///
#[derive(PartialEq, Debug, Clone)]
pub enum Message {
    Handshake(Handshake),
    //    BITFIELD(Bitfield),
    //    EXTENDED(Extended),
    Have(Have),
    //Piece(Block),
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Request,
    /// Cancel(index, begin, length), used to cancel block reques
    /// TODO : Create Cancel Struct
    Cancel(Cancel),
    Port(u16),
}

/// Cancel :
///
/// It's a fixed length message and used to cancel block requests.
/// The payload is identical to that of the "request" message
/// It is typically used during "End Game".
///
/// Structure :
///
/// <len=0013><id=8><index><begin><length>
///
///  index - A u32 integer specifying the zero based piece index
///  begin - A u32 integer specifying the zero based byte offset within the piece
///  length - A u32 integer specifying the requested length
///
/// It has a total length of 4 + 13 = 17 bytes
#[derive(PartialEq, Debug, Clone)]
pub struct Cancel {
    index: u32,
    begin: u32,
    length: u32,
}

impl Cancel {
    /// Creates a Cancel Struct from the bytes of Cancel Message Frame
    ///
    /// src - It must be 17 bytes long
    pub fn from_bytes(src: &BytesMut) -> Self {
        // Currenlty there is no error handling
        // TODO : Add some sort of error handling
        let index_bytes = &src[5..=8];
        let begin_bytes = &src[9..=12];
        let length_bytes = &src[13..=16];

        let index = ReadBytesExt::read_u32::<BigEndian>(&mut index_bytes).unwrap();
        let begin = ReadBytesExt::read_u32::<BigEndian>(&mut begin_bytes).unwrap();
        let length = ReadBytesExt::read_u32::<BigEndian>(&mut length_bytes).unwrap();

        Self { index, begin, length }
    }
}

///// INTERESTED message
/////
///// INTERESTED Message is sent after HANDSHAKE message has been
///// exchanged between the peers, its used to tell the peer "hey i'm interested in exchanging files with you",
///// and the peer can choose choke or unchoke us by sending us CHOKE or UNCHOKE message,
///// we can then choose to CHOKE and UNCHOKE the user after by looking at the response of this INTERESTED message
/////
///// NOTE : Sending an INTERESTED message doesn't guarantee that the peer would send UNCHOKE
//pub struct Interested;
//
//impl Interested {
//    pub fn build_message() -> BytesMut {
//        let mut bytes_mut = BytesMut::new();
//        bytes_mut.put_u32(1);
//        bytes_mut.put_u8(2);
//        bytes_mut
//    }
//}
//
//
///// Request Message
/////
///// length_prefix => 13u32 (Total length of the payload that follows the initial 4 bytes)
///// id => u8 (id of the message)
///// index => (index of the piece)
///// begin => (index of the beginning byte)
///// length => (length of the piece from beginning offset)
//pub struct Request {
//    length_prefix: u32,
//    id: u8,
//    index: u32,
//    begin: u32,
//    length: u32,
//}
//
//impl Request {
//    // TODO : Remove all parameters and make it BytesMut
//    pub fn from(index: u32, begin: u32, length: u32) -> Self {
//        //let mut buf = BytesMut::new();
//        Self {
//            length_prefix: 13,
//            id: 6,
//            index,
//            begin,
//            length,
//        }
//    }
//
//    pub fn build_message(index: u32, begin: u32, length: u32) -> BytesMut {
//        let length_prefix = 13;
//        let id = 6;
//        let mut bytes_mut: BytesMut = BytesMut::new();
//        bytes_mut.put_u32(length_prefix);
//        bytes_mut.put_u8(id);
//        bytes_mut.put_u32(index);
//        bytes_mut.put_u32(begin);
//        bytes_mut.put_u32(length);
//        bytes_mut
//    }
//}
//
//#[derive(Debug, Clone, PartialEq)]
//pub struct Bitfield {
//    pub have: Vec<u32>,
//    pub not_have: Vec<u32>,
//}
//
//impl Bitfield {
//    pub fn from(bytes: &mut BytesMut) -> crate::Result<Self> {
//        let mut have: Vec<u32> = Vec::new();
//        let mut not_have: Vec<u32> = Vec::new();
//
//        let length_prefix: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[0..=3]).unwrap();
//        let bitfield_total_length = (length_prefix + 4) as usize;
//
//        // If the bytes size is less than bitfield total length then it means it needs more tcp
//        // packet to combine and create its full data
//        if bytes.len() < bitfield_total_length {
//            return Err("WAIT FOR MORE TCP SEGMENT".into());
//        } else {
//            let bitfield_payload = bytes.split_to(bitfield_total_length);
//            for i in 0..bitfield_payload.len() - 1 {
//                match bitfield_payload[i] {
//                    0 => not_have.push(i as u32),
//                    1 => have.push(i as u32),
//                    _ => {}
//                }
//            }
//        }
//
//        Ok(Self { have, not_have })
//    }
//}
//
///// Extended Message :
/////
///// A message type for those who implement the Extension Protocol
///// from - http://www.bittorrent.org/beps/bep_0010.html
/////
///// Structure :
///// length_prefix => u32 (No of bytes for the entire message) : Offset [0,3]
///// message_id => u8 (value = 20) (id of the message) : Offset : [4]
///// extension_message_id => u8 (id of the message) : Offset : [5]
//#[derive(Debug, Clone, PartialEq)]
//pub struct Extended {
//    length_prefix: u32,
//    message_id: u8,
//    extension_message_id: u8,
//    payload: ExtendedPayload,
//}
//
//impl Extended {
//    pub fn from(bytes: &mut BytesMut) -> Self {
//        let length_prefix: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[0..=3]).unwrap();
//        let message_id: u8 = bytes[4];
//        let extension_message_id: u8 = bytes[5];
//        bytes.split_to(6);
//
//        let payload_length = (length_prefix - 2) as usize;
//        let payload = bytes.split_to(payload_length);
//        let payload: ExtendedPayload = serde_bencode::de::from_bytes(&payload).unwrap();
//
//        Extended {
//            length_prefix,
//            message_id,
//            extension_message_id,
//            payload,
//        }
//    }
//}
//
//#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
//struct ExtendedPayload {
//    pub v: Option<String>,
//}
//
//
/// HAVE Message :
/// TODO : Add info about HAVE message
#[derive(Debug, PartialEq, Clone)]
pub struct Have {
    pub piece_index: u32,
}

impl Have {
    pub fn from_bytes(src: &mut BytesMut) -> Self {
        let piece_index_bytes = &src[5..=8];
        // TODO : Make sure unpwrap here is safe
        let piece_index = ReadBytesExt::read_u32::<BigEndian>(&mut piece_index_bytes).unwrap();
        Self { piece_index }
    }
}

/// Handshake Message :
///
/// It's the first message to be exchanged by us (the initiator of the connection), with the peer.
/// A Handshake message has fixed 68 byte length. The peer also sends HANDSHAKE as the first
/// message to us.
///
/// NOTE : If any peer sends us message other than Handshake as the first message when we send
/// them Handshake message, then we must terminate the Connection.
///
///
/// NOTE : It drops the connection as soon as it sees CHOKE message as a response
/// of the HANDSHAKE message
///
/// NOTE : If a peer sends a Handshake message with different info hash, then we are suppose to
/// drop the connection right there. Even if we send a different info hash in the Handshake
/// message, there is always a chance that connection is to be dropped.
///
///

/// Structure :
/// Handshake : <pstrlen><pstr><reserved><info_hash><peer_id>  WHERE
///
/// - pstrlen : String length of <pstr>, as a single raw byte, in BitTorrent V1, pstrlen = 19
/// - pstr : String identifier of the protocol, in BitTorrent V1, pstr = "BitTorrent protocol"
/// - reserved : Eight(8) reserved bytes, which is all zeroes. Each bit in this field can be
///              used to changte the behaviour of the protocol
/// - info_hash : 20 byte SHA1 hash of the info key in the metainfo file i.e ".torrent" file
/// - peer_id : 20 byte String, used as a unique ID for the client. This is usually the same
///             peer_id transmitted in tracker requests(not always, there is an anonymitiy
///             option in Azureus BitTorrent Client)
///
///
/// For More : https://wiki.theory.org/indx.php/BitTorrentSpecification#Handshakee
///
#[derive(Debug, Clone, PartialEq)]
pub struct Handshake {
    pstrlen: u8,
    pstr: Vec<u8>,
    reserved: Vec<u8>,
    info_hash: Vec<u8>,
    peer_id: Vec<u8>,
}

impl Handshake {
    /// Creates a instance of Handshake in order to send it to a peer.
    pub fn new(state: Arc<State>) -> Self {
        let pstrlen: u8 = 19;
        let pstr = b"BitTorrent protocol".to_vec();
        let reserved = vec![0; 8];
        let info_hash = state.info_hash.clone();
        let peer_id = b"-HYBLOW-110011001100".map(|v| v).into_iter().collect();
        // TODO : Create a Peer Id field in state field and use that field here
        Self {
            pstrlen,
            pstr,
            reserved,
            info_hash,
            peer_id,
        }
    }

    /// Deserializes given bytes into Handshake instance
    pub fn from(v: &mut BytesMut) -> Self {
        let pstrlen = v.split_to(1).to_vec()[0];
        let pstr = v.split_to(19).to_vec();
        let reserved = v.split_to(8).to_vec();
        let info_hash = v.split_to(20).to_vec();
        let peer_id = v.split_to(20).to_vec();

        Self {
            pstrlen,
            pstr,
            reserved,
            info_hash,
            peer_id,
        }
    }

    /// Serializes the Handshake instance into bytes
    pub fn to_bytes(&self) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_u8(self.pstrlen);
        buf.put_slice(&(self.pstr));
        buf.put_slice(&(self.reserved));
        buf.put_slice(&(self.info_hash));
        buf.put_slice(&(self.peer_id));
        buf
    }
}

/// Unchoke message
///
/// Structure :
///
/// <len=0001><id=1>
/// (Length Prefix)(Message ID)
///
/// TODO: Write somethig about Unchoke message
///
pub struct Unchoke;

impl Unchoke {
    pub fn to_bytes() -> BytesMut {
        let mut bytes_mut = BytesMut::new();
        bytes_mut.put_u32(1);
        bytes_mut.put_u8(1);
        bytes_mut
    }
}

/// Interested message
///
/// Structure :
///
/// <len=0001><id=2>
/// (Length Prefix)(Message ID)
///
/// Interested Message is sent after Handshake message has been exchanged between the
/// peers, its used to tell the peer "hey i'm interested in exchanging files with you",
/// and the peer can choose choke or unchoke us by sending us CHOKE or UNCHOKE message,
/// we can then choose to CHOKE and UNCHOKE the user after by looking at the response of
/// this Interested message
///
/// NOTE : Sending an Interested message doesn't guarantee that the peer would send UNCHOKE
///
pub struct Interested;

impl Interested {
    pub fn to_bytes() -> BytesMut {
        let mut bytes_mut = BytesMut::new();
        bytes_mut.put_u32(1); // Length Prefix
        bytes_mut.put_u8(2); // Message ID
        bytes_mut
    }
}

/// Request Message
///
/// Structure :
///
/// <len=0013><id=6><index><begin><length>
/// (Length Prefix)(Message ID)(0 based piece index)(0 based byte offset)(Requested length)
///
/// length_prefix => 13u32 (Total length of the payload that follows the initial 4 bytes)
/// id => u8 (id of the message)
/// index => (index of the piece)
/// begin => (index of the beginning byte)
/// length => (length of the piece from beginning offset)
///
pub struct Request {
    length_prefix: u32,
    id: u8,
    index: u32,
    begin: u32,
    length: u32,
}
//
//impl Request {
//    // TODO : Remove all parameters and make it BytesMut
//    pub fn from(index: u32, begin: u32, length: u32) -> Self {
//        //let mut buf = BytesMut::new();
//        Self {
//            length_prefix: 13,
//            id: 6,
//            index,
//            begin,
//            length,
//        }
//    }
//
//    pub fn build_message(index: u32, begin: u32, length: u32) -> BytesMut {
//        let length_prefix = 13;
//        let id = 6;
//        let mut bytes_mut: BytesMut = BytesMut::new();
//        bytes_mut.put_u32(length_prefix);
//        bytes_mut.put_u8(id);
//        bytes_mut.put_u32(index);
//        bytes_mut.put_u32(begin);
//        bytes_mut.put_u32(length);
//        bytes_mut
//    }
//}
