/*
 * NOTE : This file contains all the structs and methods related to
 * Bittorent Message
 *
 * All the messages and their specified protocol is taken from :
 * https://wiki.theory.org/index.php/BitTorrentSpecification#Messages
 * https://www.bittorrent.org/beps/bep_0010.html
 * Initially we as a peer start as :
 *
 * NOT_INTERESTED and
 * CHOKE
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

/// Messages sent to the peer and recieved form the peer takes
/// the following forms
#[derive(PartialEq, Debug, Clone)]
pub enum Message {
    Handshake(Handshake),
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Bitfield(Bitfield),
    //    EXTENDED(Extended)
    Have(Have),
    Request(Request),
    Piece(Block),
    Cancel(Cancel),
    Port(Port),
}

impl Message {
    /// Converts a Message into Bytes
    pub fn to_bytes(&self) -> BytesMut {
        let mut buf = BytesMut::new();
        match *self {
            Message::Handshake(ref handshake) => {
                buf.put_u8(handshake.pstrlen);
                buf.put_slice(&handshake.pstr);
                buf.put_slice(&handshake.reserved);
                buf.put_slice(&handshake.info_hash);
                buf.put_slice(&handshake.peer_id);
            }

            Message::KeepAlive => {
                buf.put_u32(0);
            }

            Message::Choke => {
                buf.put_u32(1);
                buf.put_u8(1);
            }

            Message::Unchoke => {
                buf.put_u32(1);
                buf.put_u8(2);
            }

            Message::Interested => {
                buf.put_u32(1);
                buf.put_u8(3);
            }

            Message::NotInterested => {
                buf.put_u32(1);
                buf.put_u8(4);
            }
            // TODO : Do it for all messages

            //Message::Have(ref have) => {}
            _ => {}
        }
        return buf;
    }

    /// Checks in the given src buffer if the first Message Frame is A Handshake Message Frame
    pub fn is_handshake_message(src: &BytesMut) -> bool {
        let _expected_pstr = String::from("BitTorrent protocol");
        // First check if there is enough bytes to be a Handshake Message Frame
        if src.len() >= 68 {
            let pstr_len = src[0];
            // TODO : Check pstr
            // let pstr: Vec<u8> = [1..=19].iter().collect();
            //let pstr = String::from_utf8(pstr).unwrap();
            if pstr_len == 19 {
                return true;
                //   return pstr == expected_pstr;
                // TODO : Check for info hash
                // TODO : Check the peer id
                // Then only validate the Handshake Message Frame
            }
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A KeepAlive Message Frame
    ///
    /// Structure :
    ///
    /// <0000>
    ///
    /// Which just contains the length prefix and does not have any id and
    /// payload
    ///
    /// Steps :
    /// Firstly we check if there are enough bytes to contain the length_prefix field i.e <0000>
    /// field of the KeepAlive Message, if it does then and we need to deserialize those
    /// length_prefix to u32, if the value is 0_u32 then it's a KeepAlive Message.
    ///
    /// Because, no other Message Frame has a message that starts with bytes 0000, so there is no chance
    /// for this collide with other messages as far as i know
    ///
    pub fn is_keep_alive_message(src: &BytesMut) -> bool {
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();
            return length_prefix == 0;
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A Choke Message Frame
    pub fn is_choke_message(src: &BytesMut) -> bool {
        // First check if there is enough bytes to get the length prefix
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();
            // Once we get the length prefix, we check if its value is equal to 1, and has enough
            // bytes for the entire Choke Message Frame and the message id is 0
            if length_prefix == 0 && src.len() >= 5 {
                let message_id = src[4];
                return message_id == 0;
            }
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A Unchoke Message Frame
    pub fn is_unchoke_message(src: &BytesMut) -> bool {
        // First check if there is enough bytes to get the length prefix
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();
            // Once we get the length prefix, we check if its value is equal to 1, and has enough
            // bytes for the entire Unchoke Message Frame and the message id is 1
            if length_prefix == 0 && src.len() >= 5 {
                let message_id = src[4];
                return message_id == 1;
            }
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A Interested Message Frame
    pub fn is_interested_message(src: &BytesMut) -> bool {
        // First check if there is enough bytes to get the length prefix
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();
            // Once we get the length prefix, we check if its value is equal to 1, and has enough
            // bytes for the entire Unchoke Message Frame and the message id is 2
            if length_prefix == 0 && src.len() >= 5 {
                let message_id = src[4];
                return message_id == 2;
            }
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A NotInterested Message Frame
    pub fn is_not_interested_message(src: &BytesMut) -> bool {
        // First check if there is enough bytes to get the length prefix
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();
            // Once we get the length prefix, we check if its value is equal to 1, and has enough
            // bytes for the entire Unchoke Message Frame and the message id is 3
            if length_prefix == 0 && src.len() >= 5 {
                let message_id = src[4];
                return message_id == 3;
            }
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A Have Message Frame
    pub fn is_have_message(src: &BytesMut) -> bool {
        // First check if there is enough bytes to get the length prefix
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();

            // Once we get the length prefix, then we simply check if the length_prefix matches
            // or not and the source is atleast (4 + length_prefix) i.e (4 + 5) bytes or not
            if length_prefix == 5 && src.len() >= 9 {
                let message_id = src[4];
                return message_id == 4;
            }
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A Bitfield Message Frame
    pub fn is_bitfield_message(src: &BytesMut) -> bool {
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();

            // Expected length of the entire Bitfield Message Frame
            let expected_frame_length = 4 + length_prefix;

            // Second, check if there is enough data in the buffer
            // as mentioned in length_prefix
            if src.len() as u32 >= expected_frame_length {
                let message_id = src[4];
                return message_id == 5;
            }
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A Request Message Frame
    pub fn is_request_message(src: &BytesMut) -> bool {
        // First check if there is enough bytes to get the length prefix
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();

            // Once we get the length prefix, then we simply check if the source is atleast
            // (4 + lengthprefix) bytes or not
            if length_prefix == 13 && src.len() >= 17 {
                let message_id = src[4];
                return message_id == 6;
            }
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A Piece Message Frame
    pub fn is_piece_message(src: &BytesMut) -> bool {
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();

            // Expected length of the entire message
            let expected_length = 4 + length_prefix;
            // Second, check if there is enough data in the buffer
            // as mentioned in length_prefix
            if src.len() as u32 >= expected_length {
                let message_id = src[4];
                return message_id == 7;
            }
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A Cancel Message Frame
    pub fn is_cancel_message(src: &BytesMut) -> bool {
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();

            // Once we get the length prefix, then we simply check if the source is atleast
            // (4 + lengthprefix) bytes or not
            if length_prefix == 13 && src.len() >= 17 {
                let message_id = src[4];
                return message_id == 8;
            }
        }
        return false;
    }

    /// Checks in the given src buffer if the first Message Frame is A Port Message Frame
    pub fn is_port_message(src: &BytesMut) -> bool {
        if src.len() >= 4 {
            let mut length_prefix_bytes = &src[0..=3];
            let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();

            // Once we get the length prefix, then we simply check if the source is atleast
            // (4 + lengthprefix) bytes or not
            if length_prefix == 3 && src.len() >= 7 {
                let message_id = src[4];
                return message_id == 9;
            }
        }
        return false;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bitfield {
    /// Represents the pieces that peer have
    pub have: Vec<usize>,

    /// Represents the pieces that peer doesn't have
    pub not_have: Vec<usize>,
}

impl Bitfield {
    pub fn from_bytes(src: &mut BytesMut) -> Self {
        let mut have = Vec::new();
        let mut not_have = Vec::new();

        let mut length_prefix_bytes = &src[0..=3];
        let length_prefix = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();

        let bitfield_frame_length = (length_prefix + 4) as usize;

        let bitfield_bytes = &src[5..bitfield_frame_length];
        let bitfield = BytesMut::from(bitfield_bytes);

        for (index, bit) in bitfield.iter().enumerate() {
            match bit {
                0 => not_have.push(index),
                1 => have.push(index),
                _ => {}
            }
        }
        src.split_to(bitfield_frame_length as usize);
        Self {
            have,
            not_have,
        }
    }
}

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
        let mut piece_index_bytes = &src[5..=8];
        // TODO : Make sure unpwrap here is safe
        let piece_index = ReadBytesExt::read_u32::<BigEndian>(&mut piece_index_bytes).unwrap();
        Self {
            piece_index,
        }
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

        v.split_to(68);
        Self {
            pstrlen,
            pstr,
            reserved,
            info_hash,
            peer_id,
        }
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

/// Request Message :
///
/// A Request Message is a fixed length message,
/// which is used to request a block.
///
/// Structure :
///
/// <len=0013><id=6><index><begin><length>
///
///  index - A u32 integer, which specifies the zero based piece index
///  begin - A u32 integer, which specifies the zero based byte offset within the piece
///  length - A u32 integer, which specifies the requested length
///
/// It has a total frame length of (4 + 13) bytes
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// Zero based piece index
    index: u32,
    /// Zero based byte offset within the piecec
    begin: u32,
    /// Requested length
    length: u32,
}

impl Request {
    /// Creates a Request instance from the Request Message Frame bytes.
    /// It consumes the frame bytes and produces an instance of Request
    ///
    /// src - It must be validated by using is_request_message() method
    /// before calling this from_bytes() method
    pub fn from_bytes(src: &mut BytesMut) -> Self {
        let mut index_bytes = &src[5..=8];
        let mut begin_bytes = &src[9..=12];
        let mut length_bytes = &src[13..=16];

        let index = ReadBytesExt::read_u32::<BigEndian>(&mut index_bytes).unwrap();
        let begin = ReadBytesExt::read_u32::<BigEndian>(&mut begin_bytes).unwrap();
        let length = ReadBytesExt::read_u32::<BigEndian>(&mut length_bytes).unwrap();

        src.split_to(17);
        Self {
            index,
            begin,
            length,
        }
    }
}

/// Piece Message :
///
/// A Block Message, also coined as Piece Message(Not the same thing)
/// A Block is a subset of a Piece. Whenever we request some data from a peer,
/// we don't directly request a piece, but rather request the blocks that make
/// up that piece
///
/// It's a variable length message, where X is the length of the block.
///
/// Structure :
///
/// <len=0009+X><id=7><index><begin><block>
///
///  index - A u32 integer, which specifies the zero based piece index
///  begin - A u32 integer, which specifies the zero based byte offset within the piece
///  block - Bytes, which is the block of data that make up the piece
///
/// It has a total frame length of (4 + 9 + X) bytes, where X is the length of the block
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Zero based piece index
    pub piece_index: u32,
    /// Zero based byte index within the piece
    pub byte_index: u32,
    /// Block of raw data(bytes)
    pub raw_block: BytesMut,
}

impl Block {
    //  /// Creates a "Block" instance from the raw "Piece" message sent by the client
    //  /// NOTE : It removes the data it read from the buffer
    pub fn from_bytes(src: &mut BytesMut) -> Self {
        let mut length_prefix_bytes = &src[0..=3];
        let mut piece_index_bytes = &src[5..=8];
        let mut bytes_index_bytes = &src[9..=12];

        let length_prefix: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut length_prefix_bytes).unwrap();
        let piece_index: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut piece_index_bytes).unwrap();
        let byte_index: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut bytes_index_bytes).unwrap();

        let block_length = (length_prefix - 9) as usize;
        let block_bytes = &src[13..block_length];
        let raw_block = BytesMut::from(block_bytes);

        let total_frame_length = 4 + length_prefix;
        src.split_to(total_frame_length as usize);
        Self {
            piece_index,
            byte_index,
            raw_block,
        }
    }
}

/// Cancel Message :
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
/// It has a total frame length of 4 + 13 = 17 bytes
#[derive(PartialEq, Debug, Clone)]
pub struct Cancel {
    index: u32,
    begin: u32,
    length: u32,
}

impl Cancel {
    /// Creates a Cancel instance from the bytes of Cancel Message Frame
    /// src - It must be atleast 17 bytes long and must be validated by
    /// Cancel::is_cancel_message() function before actually creating it
    ///
    /// It will consume the Cancel Message Frame bytes and create the Cancel instance
    pub fn from_bytes(src: &mut BytesMut) -> Self {
        // TODO : Add some sort of error handling by using is_cancel_message() method
        let mut index_bytes = &src[5..=8];
        let mut begin_bytes = &src[9..=12];
        let mut length_bytes = &src[13..=16];

        let index = ReadBytesExt::read_u32::<BigEndian>(&mut index_bytes).unwrap();
        let begin = ReadBytesExt::read_u32::<BigEndian>(&mut begin_bytes).unwrap();
        let length = ReadBytesExt::read_u32::<BigEndian>(&mut length_bytes).unwrap();

        src.split_to(17);
        Self {
            index,
            begin,
            length,
        }
    }
}

/// Port Message :
///
/// It's a fixed length message and sent by the newer version of Mainline that implements
/// the DHT Tracker. The listen port is this peer's DHS node is listening.
/// This peer should be insertedin the local routing table (if DHT Tracker is supported)
///
/// Structure :
///
/// <len=0003><id=9><listen-port>
///
/// listen-port : A u16 integer, specifying port that peer's DHT node is listening on
///
/// It has a total length of 4 + 3 = 7 bytes
#[derive(PartialEq, Debug, Clone)]
pub struct Port {
    listen_port: u16,
}

impl Port {
    /// Creates a Port instance from the bytes of Port Message Frame
    /// src - It must be atleast 7 bytes long and must be validated by
    /// Port::is_port_message() function before actually creating it.
    ///
    /// It will consume the Port Message Frame bytes and create the Port instance
    pub fn from_bytes(src: &mut BytesMut) -> Self {
        // TODO : Add some sort of error handling by using is_por_message() method
        let mut listen_port_bytes = &src[4..=6];

        let listen_port = ReadBytesExt::read_u16::<BigEndian>(&mut listen_port_bytes).unwrap();
        src.split_to(7);
        Self {
            listen_port,
        }
    }
}
