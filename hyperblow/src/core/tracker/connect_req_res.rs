use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use rand::Rng;

/// Struct to handle the message to be sent to "Connect" on the UDP Tracker
/// Used to create a "16 byte" buffer to make "Connect Request"
///
/// Connect Request Bytes Structure:
///
/// Offset  Size            Name            Value
/// 0       64-bit integer  protocol_id     0x41727101980 // magic constant
/// 8       32-bit integer  action          0 // connect
/// 12      32-bit integer  transaction_id  A random number
/// 16
///
#[derive(Debug, Clone,)]
pub struct ConnectRequest {
    pub protocol_id: i64,
    pub action: i32,
    pub transaction_id: i32,
}

impl ConnectRequest {
    // Creates a fresh ConnectRequest instance
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            protocol_id: 0x41727101980,
            action: 0,
            transaction_id: rng.gen(),
        }
    }

    // Gives a buffer view of the ConnectRequest
    pub fn serializeToBytes(&self,) -> BytesMut {
        let mut bytes = BytesMut::with_capacity(16,);
        bytes.put_i64(self.protocol_id,);
        bytes.put_i32(self.action,);
        bytes.put_i32(self.transaction_id,);
        bytes
    }
}

/// Struct to handle response message from "Connect" request to the UDP Tracker
/// Used to create an instance of AnnounceRequest
/// Connect Response Bytes Structure from the UDP Tracker Protocol :
///
/// Offset  Size            Name            Value
/// 0       32-bit integer  action          0 // connect
/// 4       32-bit integer  transaction_id
/// 8       64-bit integer  connection_id
/// 16
#[derive(Debug, Clone,)]
pub struct ConnectResponse {
    pub action: i32,
    pub transaction_id: i32,
    pub connection_id: i64,
}

impl ConnectResponse {
    pub fn from(v: &[u8],) -> Result<ConnectResponse, std::io::Error,> {
        let mut action_bytes = &v[0..=3];
        let mut transaction_id_bytes = &v[4..=7];
        let mut connection_id_bytes = &v[8..=15];

        let action = ReadBytesExt::read_i32::<BigEndian,>(&mut action_bytes,)?;
        let transaction_id = ReadBytesExt::read_i32::<BigEndian,>(&mut transaction_id_bytes,)?;
        let connection_id = ReadBytesExt::read_i64::<BigEndian,>(&mut connection_id_bytes,)?;
        Ok(Self {
            action,
            transaction_id,
            connection_id,
        },)
    }
}
