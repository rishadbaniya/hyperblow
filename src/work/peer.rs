use bytes::{BufMut, BytesMut};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
//
// Protocol Implemented From :  https://wiki.theory.org/indx.php/BitTorrentSpecification#Handshakee
// In version 1.0 of the BitTorrent protocol, pstrlen = 19, and pstr = "BitTorrent protocol".

// 0. pstrlen => Single byte value, i.e u8 (Value = 19)
// 1. pstr => String identifier of the protocol (Value = "BitTorrent protocol" )
// 2. reserved => 8 reserved bytes. Current implentation uses all zeroes
// 3.
struct HandshakeRequest {
    pstrlen: u8,
    pstr: Vec<u8>,
    reserved: Vec<u8>,
    peer_id: Option<Vec<u8>>,
    info_hash: Option<Vec<u8>>,
}

impl HandshakeRequest {
    fn new() -> Self {
        let pstr: Vec<u8> = b"BitTorrent protocol".map(|v| v).into_iter().collect();
        let reserved = vec![0; 8];
        let pstrlen: u8 = 19;
        HandshakeRequest {
            pstrlen,
            pstr,
            reserved,
            info_hash: None,
            peer_id: None,
        }
    }
    fn getBytesMut(&self) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_u8(self.pstrlen);
        buf.put_slice(self.pstr.as_slice());
        buf.put_slice(self.reserved.as_slice());
        buf.put_slice(self.peer_id.as_ref().unwrap().as_slice());
        buf.put_slice(self.info_hash.as_ref().unwrap().as_slice());
        buf
    }
}

//
// PEER REQUEST (TCP)
//
// Objective : Connect to Peers and download pieces(blocks)
//
// First of all, we make a TCP connection with the "peer", after making TCP connection with the p
pub async fn peer_request(socket_adr: SocketAddr) {
    const CONNECTION_TIMEOUT: u64 = 15;
    loop {
        match timeout(Duration::from_secs(CONNECTION_TIMEOUT), TcpStream::connect(socket_adr)).await {
            Ok(v) => match v {
                Ok(mut stream) => {
                    let handshake_request = HandshakeRequest::new();
                    if let Ok(_) = stream.write_all(b"hey brother").await {
                        println!("I WROTE IT ALL");
                        if let Ok(_) = stream.readable().await {
                            println!("I CAN READ NOW");
                            let mut buf: Vec<u8> = Vec::new();
                            stream.read(&mut buf).await;
                            println!("{:?}", buf);
                        }
                    }

                    sleep(Duration::from_secs(10)).await
                }
                Err(e) => {
                    // Connection Refused or Something related with Socket address
                    sleep(Duration::from_secs(240)).await
                }
            },
            Err(e) => {
                // Timeout Error
                sleep(Duration::from_secs(240)).await
            }
        }
    }
}
