#![allow(non_camel_case_types)]
#![allow(unused_must_use)]

use crate::Result;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::BytesMut;

/// Stores Block sent by peer as a response to "Request" message
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Length of the data that follows "length" field
    pub length: u32,
    /// Message id (Default value = 6)
    pub message_id: u8,
    /// Zero based piece index
    pub piece_index: u32,
    /// Byte index from which data is started to send, zero based byte index within the piece
    pub byte_index: u32,
    /// Block of raw data(bytes)
    pub raw_block: BytesMut,
}

impl Block {
    /// Creates a "Block" instance from the raw "Piece" message sent by the client
    /// NOTE : It removes the data it read from the buffer
    pub fn from(bytes: &mut BytesMut) -> Result<Self> {
        let length: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[0..=3])?;
        if (length + 4) as usize != bytes.len() {
            return Err("NEED MOER TCP SEGMENT".into());
        }
        let message_id: u8 = bytes[4];
        let piece_index: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[5..=8])?;
        let byte_index: u32 = ReadBytesExt::read_u32::<BigEndian>(&mut &bytes[9..=12])?;
        // Strip away the total no of initial bytes that are not part of the raw block we want
        let total_non_raw_block_bytes = 13;
        bytes.split_to(13);
        let raw_block = bytes.clone();
        bytes.clear();

        Ok(Self {
            length,
            message_id,
            piece_index,
            byte_index,
            raw_block,
        })
    }
}
