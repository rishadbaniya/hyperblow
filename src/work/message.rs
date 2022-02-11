use bytes::{BufMut, BytesMut};

// Interested message
pub struct Interested;

impl Interested {
    pub fn build_message() -> BytesMut {
        let mut bytes_mut = BytesMut::new();
        bytes_mut.put_u32(1);
        bytes_mut.put_u8(2);
        bytes_mut
    }
}

// Unchoke Message
pub struct Unchoke;

impl Unchoke {
    pub fn build_message() -> BytesMut {
        let mut bytes_mut = BytesMut::new();
        bytes_mut.put_u32(1);
        bytes_mut.put_u8(1);
        bytes_mut
    }
}

//
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
