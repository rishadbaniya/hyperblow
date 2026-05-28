use super::{
    messages::{Bitfield, Block, Cancel, ExtendedMessage, Handshake, Have, Port, Request},
    Message,
};
use bytes::{BufMut, BytesMut};
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};

const MAX_PEER_FRAME_LENGTH: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub struct PeerMessageCodec;

#[derive(Debug, Error)]
pub enum PeerCodecError {
    #[error("peer frame length {length} exceeds configured maximum {max}")]
    FrameTooLarge { length: usize, max: usize },

    #[error("invalid peer message frame: {0}")]
    InvalidFrame(&'static str),

    #[error("io error")]
    Io(#[from] std::io::Error),
}

impl Decoder for PeerMessageCodec {
    type Item = Message;
    type Error = PeerCodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        if src[0] == 19 {
            if src.len() < 68 {
                return Ok(None);
            }
            if Message::is_handshake_message(src) {
                return Ok(Some(Message::Handshake(Handshake::from(src))));
            }
            return Err(PeerCodecError::InvalidFrame("invalid BitTorrent handshake"));
        }

        if src.len() < 4 {
            return Ok(None);
        }

        let length_prefix = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;
        if length_prefix > MAX_PEER_FRAME_LENGTH {
            return Err(PeerCodecError::FrameTooLarge {
                length: length_prefix,
                max: MAX_PEER_FRAME_LENGTH,
            });
        }

        let frame_length = 4 + length_prefix;
        if src.len() < frame_length {
            return Ok(None);
        }

        if length_prefix == 0 {
            let _ = src.split_to(4);
            return Ok(Some(Message::KeepAlive));
        }

        let message_id = src[4];
        match message_id {
            0 if length_prefix == 1 => {
                let _ = src.split_to(frame_length);
                Ok(Some(Message::Choke))
            }
            1 if length_prefix == 1 => {
                let _ = src.split_to(frame_length);
                Ok(Some(Message::Unchoke))
            }
            2 if length_prefix == 1 => {
                let _ = src.split_to(frame_length);
                Ok(Some(Message::Interested))
            }
            3 if length_prefix == 1 => {
                let _ = src.split_to(frame_length);
                Ok(Some(Message::NotInterested))
            }
            4 if length_prefix == 5 => Ok(Some(Message::Have(Have::from_bytes(src)))),
            5 if length_prefix >= 1 => Ok(Some(Message::Bitfield(Bitfield::from_bytes(src)))),
            6 if length_prefix == 13 => Ok(Some(Message::Request(Request::from_bytes(src)))),
            7 if length_prefix >= 9 => Ok(Some(Message::Piece(Block::from_bytes(src)))),
            8 if length_prefix == 13 => Ok(Some(Message::Cancel(Cancel::from_bytes(src)))),
            9 if length_prefix == 3 => Ok(Some(Message::Port(Port::from_bytes(src)))),
            20 if length_prefix >= 2 => Ok(Some(Message::Extended(ExtendedMessage::from_bytes(src)))),
            _ => Err(PeerCodecError::InvalidFrame("unknown message id or invalid length prefix")),
        }
    }
}

impl Encoder<Vec<Message>> for PeerMessageCodec {
    type Error = PeerCodecError;
    fn encode(&mut self, item: Vec<Message>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        for ref message in item {
            dst.put(message.to_bytes());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{PeerCodecError, PeerMessageCodec, MAX_PEER_FRAME_LENGTH};
    use crate::core::peer::{messages::ExtendedMessage, Message};
    use bytes::{BufMut, BytesMut};
    use tokio_util::codec::{Decoder, Encoder};

    #[test]
    fn decodes_choke_and_consumes_frame() {
        let mut codec = PeerMessageCodec;
        let mut src = BytesMut::from(&[0, 0, 0, 1, 0][..]);

        let message = codec.decode(&mut src).expect("valid frame").expect("message");

        assert_eq!(message, Message::Choke);
        assert!(src.is_empty());
    }

    #[test]
    fn waits_for_incomplete_length_prefixed_frame() {
        let mut codec = PeerMessageCodec;
        let mut src = BytesMut::from(&[0, 0, 0, 5, 4, 0, 0][..]);

        assert!(codec.decode(&mut src).expect("partial frame should not error").is_none());
        assert_eq!(src.as_ref(), &[0, 0, 0, 5, 4, 0, 0]);
    }

    #[test]
    fn rejects_oversized_peer_frame() {
        let mut codec = PeerMessageCodec;
        let mut src = BytesMut::new();
        src.put_u32((MAX_PEER_FRAME_LENGTH + 1) as u32);

        let error = codec.decode(&mut src).expect_err("oversized frame should fail");

        assert!(matches!(error, PeerCodecError::FrameTooLarge { .. }));
    }

    #[test]
    fn rejects_unknown_complete_frame() {
        let mut codec = PeerMessageCodec;
        let mut src = BytesMut::from(&[0, 0, 0, 1, 99][..]);

        let error = codec.decode(&mut src).expect_err("unknown frame should fail");

        assert!(matches!(error, PeerCodecError::InvalidFrame(_)));
    }

    #[test]
    fn encodes_messages_with_standard_ids() {
        let mut codec = PeerMessageCodec;
        let mut dst = BytesMut::new();

        codec
            .encode(vec![Message::Choke, Message::Interested], &mut dst)
            .expect("messages should encode");

        assert_eq!(dst.as_ref(), &[0, 0, 0, 1, 0, 0, 0, 0, 1, 2]);
    }

    #[test]
    fn decodes_extended_message() {
        let mut codec = PeerMessageCodec;
        let mut src = BytesMut::new();
        src.put_u32(5);
        src.put_u8(20);
        src.put_u8(3);
        src.put_slice(b"abc");

        let message = codec.decode(&mut src).expect("valid frame").expect("message");

        assert_eq!(message, Message::Extended(ExtendedMessage::new(3, b"abc".to_vec())));
        assert!(src.is_empty());
    }

    #[test]
    fn encodes_extended_message() {
        let mut codec = PeerMessageCodec;
        let mut dst = BytesMut::new();

        codec
            .encode(vec![Message::Extended(ExtendedMessage::new(1, b"xyz".to_vec()))], &mut dst)
            .expect("extended message should encode");

        assert_eq!(dst.as_ref(), &[0, 0, 0, 5, 20, 1, b'x', b'y', b'z']);
    }
}
