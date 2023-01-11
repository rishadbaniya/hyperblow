/// Struct to handle "Announce" request message
/// Used to create a "98 byte" buffer to make "Announce Request"
/// Reference : http://www.bittorrent.org/beps/bep_0015.html
///
/// IPv4 announce request Bytes Structure:
/// Offset  Size    Name    Value
/// 0       64-bit integer  connection_id   The connection id acquired from establishing the connection.
/// 8       32-bit integer  action          Action. in this case, 1 for announce. See : https://www.rasterbar.com/products/libtorrent/udp_tracker_protocol.html#actions
/// 12      32-bit integer  transaction_id  Randomized by client
/// 16      20-byte string  info_hash       The info-hash of the torrent you want announce yourself in.
/// 36      20-byte string  peer_id         Your peer id. (Peer ID Convention : https://www.bittorrent.org/beps/bep_0020.html)
/// 56      64-bit integer  downloaded      The number of byte you've downloaded in this session.
/// 64      64-bit integer  left            The number of bytes you have left to download until you're finished.
/// 72      64-bit integer  uploaded        The number of bytes you have uploaded in this session.
/// 80      32-bit integer  event           0 // 0: none; 1: completed; 2: started; 3: stopped
/// 84      32-bit integer  IP address      Your ip address. Set to 0 if you want the tracker to use the sender of this UDP packet.u
/// 88      32-bit integer  key             A unique key that is randomized by the client.
/// 92      32-bit integer  num_want        The maximum number of peers you want in the reply. Use -1 for default.
/// 96      16-bit integer  port            The port you're listening on.
/// 98
#[derive(Debug, Clone)]
pub struct AnnounceRequest {
    connection_id: Option<i64>,
    action: i32,
    transaction_id: Option<i32>,
    info_hash: Option<Vec<u8>>,
    peer_id: Option<[u8; 20]>,
    downloaded: Option<i64>,
    left: Option<i64>,
    uploaded: Option<i64>,
    event: Option<i32>,
    ip_address: Option<i32>,
    key: Option<i32>,
    num_want: i32,
    port: Option<i16>,
}

impl AnnounceRequest {
    // Creates an empty Announce instance
    pub fn empty() -> Self {
        let peer_id_slice = b"-HYBxxx-yyyyyyyyyyyy";
        let mut peer_id = [0u8; 20];
        for (index, value) in peer_id_slice.iter().enumerate() {
            peer_id[index] = *value;
        }
        AnnounceRequest {
            connection_id: None,
            action: 1,
            transaction_id: None,
            info_hash: None,
            peer_id: Some(peer_id),
            downloaded: None,
            left: None,
            uploaded: None,
            event: Some(1),
            ip_address: Some(0),
            key: None,
            num_want: -1,
            port: None,
        }
    }

    // Consumes the Announce instance and gives you a Buffer of 98 bytes that you
    // can use to make Announce Request in UDP
    pub fn getBytesMut(&self) -> BytesMut {
        let mut bytes = BytesMut::with_capacity(98);
        bytes.put_i64(self.connection_id.unwrap());
        bytes.put_i32(self.action);
        bytes.put_i32(self.transaction_id.unwrap());
        bytes.put_slice(&self.info_hash.as_ref().unwrap()[..]);
        bytes.put_slice(&self.peer_id.unwrap());
        bytes.put_i64(self.downloaded.unwrap());
        bytes.put_i64(self.left.unwrap());
        bytes.put_i64(self.uploaded.unwrap());
        bytes.put_i32(self.event.unwrap());
        bytes.put_i32(self.ip_address.unwrap());
        bytes.put_i32(self.key.unwrap());
        bytes.put_i32(self.num_want);
        bytes.put_i16(self.port.unwrap());
        bytes
    }

    pub fn set_connection_id(&mut self, v: i64) {
        self.connection_id = Some(v);
    }

    pub fn set_transaction_id(&mut self, v: i32) {
        self.transaction_id = Some(v);
    }

    pub fn set_info_hash(&mut self, v: Vec<u8>) {
        self.info_hash = Some(v);
    }

    pub fn set_downloaded(&mut self, v: i64) {
        self.downloaded = Some(v);
    }

    pub fn set_uploaded(&mut self, v: i64) {
        self.uploaded = Some(v);
    }

    pub fn set_left(&mut self, v: i64) {
        self.left = Some(v);
    }

    pub fn set_port(&mut self, v: i16) {
        self.port = Some(v);
    }

    pub fn set_key(&mut self, v: i32) {
        self.key = Some(v);
    }
}

/// IPv4 announce response:
///
/// Offet      Size            Name            Value
/// 0           32-bit integer  action          1 // announce
/// 4           32-bit integer  transaction_id
/// 8           32-bit integer  interval
/// 12          32-bit integer  leechers
/// 16          32-bit integer  seeders
/// 20 + 6 * n  32-bit integer  IP address
/// 24 + 6 * n  16-bit integer  TCP port
/// 20 + 6 * Ns
///
/// Struct to handle the response received by sending "Announce" request
#[derive(Debug, Clone)]
pub struct AnnounceResponse {
    pub action: i32,
    pub transaction_id: i32,
    pub interval: i32,
    pub leechers: i32,
    pub seeders: i32,
    pub peersAddresses: Vec<SocketAddr>,
}

use std::net::{IpAddr, Ipv4Addr};
impl AnnounceResponse {
    // Consumes response buffer of UDP AnnounceRequest
    pub fn new(v: &Vec<u8>) -> Result<Self> {
        let mut action_bytes = &v[0..=3];
        let mut transaction_id_bytes = &v[4..=7];
        let mut interval_bytes = &v[8..=11];
        let mut leechers_bytes = &v[12..=15];
        let mut seeder_bytes = &v[16..=19];
        let action = ReadBytesExt::read_i32::<BigEndian>(&mut action_bytes)?;
        let transaction_id = ReadBytesExt::read_i32::<BigEndian>(&mut transaction_id_bytes)?;
        let interval = ReadBytesExt::read_i32::<BigEndian>(&mut interval_bytes)?;
        let leechers = ReadBytesExt::read_i32::<BigEndian>(&mut leechers_bytes)?;
        let seeders = ReadBytesExt::read_i32::<BigEndian>(&mut seeder_bytes)?;

        // Range where all the IP addresses and Ports are situated
        let x = 20..v.len();

        if action == 3 || (x.len() % 6) != 0 {
            return Err("Server returned error".into());
        }

        let mut peersAddresses = vec![];
        for i in x.step_by(6) {
            let port_bytes = vec![v[i + 4], v[i + 5]];
            let mut port_bytes = &port_bytes[..];
            let port = ReadBytesExt::read_i16::<BigEndian>(&mut port_bytes)?;
            let socket_adr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(v[i], v[i + 1], v[i + 2], v[i + 3])), port as u16);
            peersAddresses.push(socket_adr);
        }

        Ok(AnnounceResponse {
            action,
            transaction_id,
            interval,
            leechers,
            seeders,
            peersAddresses,
        })
    }
}
