use byteorder::{BigEndian, ReadBytesExt};
use std::error;

/// Struct to handle response message from "Connect" request to the UDP Tracker
/// Used to create an instance of AnnounceRequest
/// Connect Response Bytes Structure from the UDP Tracker Protocol :
///
/// Offset  Size            Name            Value
/// 0       32-bit integer  action          3 // error
/// 4       32-bit integer  transaction_id
/// 8       string          message
#[derive(Debug, Clone)]
pub struct ErrorResponse {
    pub action: i32,
    pub transaction_id: i32,
    pub message: String,
}

impl ErrorResponse {
    pub fn from(v: &[u8]) -> Result<ErrorResponse, Box<dyn error::Error>> {
        let mut action_bytes = &v[0..=3];
        let mut transaction_id_bytes = &v[4..=7];
        let message_bytes = v[8..(v.len())].to_vec();

        let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes).unwrap();
        let transaction_id = ReadBytesExt::read_i32::<BigEndian>(&mut transaction_id_bytes).unwrap();
        let message = String::from_utf8(message_bytes)?;

        Ok(Self {
            action,
            transaction_id,
            message,
        })
    }
}
