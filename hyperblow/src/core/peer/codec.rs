use super::messages::{Bitfield, Block, Cancel, Handshake, Have, Port, Request};
use super::Message;
use bytes::BufMut;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug)]
pub struct PeerMessageCodec;

impl Decoder for PeerMessageCodec {
    type Item = Message;
    type Error = std::io::Error;
    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        return if src.len() == 0 {
            Ok(None)
        } else if Message::is_handshake_message(src) {
            Ok(Some(Message::Handshake(Handshake::from(src))))
        } else if Message::is_keep_alive_message(src) {
            let _ = src.split_to(4);
            Ok(Some(Message::KeepAlive))
        } else if Message::is_choke_message(src) {
            let _ = src.split_to(5);
            Ok(Some(Message::Choke))
        } else if Message::is_unchoke_message(src) {
            let _ = src.split_to(5);
            Ok(Some(Message::Unchoke))
        } else if Message::is_interested_message(src) {
            let _ = src.split_to(5);
            Ok(Some(Message::Interested))
        } else if Message::is_not_interested_message(src) {
            let _ = src.split_to(5);
            Ok(Some(Message::NotInterested))
        } else if Message::is_have_message(src) {
            Ok(Some(Message::Have(Have::from_bytes(src))))
        } else if Message::is_bitfield_message(src) {
            Ok(Some(Message::Bitfield(Bitfield::from_bytes(src))))
        } else if Message::is_request_message(src) {
            Ok(Some(Message::Request(Request::from_bytes(src))))
        } else if Message::is_piece_message(src) {
            Ok(Some(Message::Piece(Block::from_bytes(src))))
        } else if Message::is_cancel_message(src) {
            Ok(Some(Message::Cancel(Cancel::from_bytes(src))))
        } else if Message::is_port_message(src) {
            Ok(Some(Message::Port(Port::from_bytes(src))))
        } else {
            return Ok(None);
        };
    }
}

impl Encoder<Vec<Message>> for PeerMessageCodec {
    type Error = std::io::Error;
    fn encode(&mut self, item: Vec<Message>, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        for ref message in item {
            dst.put(message.to_bytes());
        }
        Ok(())
    }
}
