use super::{
    messages::{ExtendedMessage, Handshake, Message},
    PeerMessageCodec,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io, net::SocketAddr, time::Duration};
use thiserror::Error;
use tokio::{net::TcpStream, time::timeout};
use tokio_util::codec::Framed;

const METADATA_BLOCK_SIZE: usize = 16 * 1024;
const METADATA_CONNECT_TIMEOUT: Duration = Duration::from_secs(12);
const METADATA_MESSAGE_TIMEOUT: Duration = Duration::from_secs(12);
const LOCAL_UT_METADATA_ID: u8 = 1;

#[derive(Debug, Error)]
pub enum MagnetMetadataError {
    #[error("magnet metadata fetch needs a 20-byte info hash, got {0}")]
    InvalidInfoHashLength(usize),

    #[error("timed out connecting to metadata peer {0}")]
    ConnectionTimeout(SocketAddr),

    #[error("failed to connect to metadata peer {addr}")]
    Connection { addr: SocketAddr, source: io::Error },

    #[error("metadata peer closed the connection")]
    PeerClosed,

    #[error("metadata peer timed out")]
    PeerTimeout,

    #[error("metadata peer returned wrong info hash")]
    InfoHashMismatch,

    #[error("metadata peer does not advertise extension protocol")]
    ExtensionProtocolUnsupported,

    #[error("metadata peer does not advertise ut_metadata")]
    UtMetadataUnsupported,

    #[error("metadata peer did not send metadata_size")]
    MetadataSizeMissing,

    #[error("metadata peer rejected piece {0}")]
    PieceRejected(usize),

    #[error("metadata response piece {piece} is out of range for {piece_count} pieces")]
    PieceOutOfRange { piece: usize, piece_count: usize },

    #[error("metadata response is malformed: {0}")]
    MalformedResponse(&'static str),

    #[error("peer codec error")]
    Codec(#[from] super::codec::PeerCodecError),

    #[error("metadata bencode error")]
    Bencode(#[from] serde_bencode::Error),
}

pub struct MagnetMetadataFetcher;

impl MagnetMetadataFetcher {
    pub async fn fetch(socket_addr: SocketAddr, info_hash: &[u8]) -> Result<Vec<u8>, MagnetMetadataError> {
        if info_hash.len() != 20 {
            return Err(MagnetMetadataError::InvalidInfoHashLength(info_hash.len()));
        }

        let tcp_stream = match timeout(METADATA_CONNECT_TIMEOUT, TcpStream::connect(socket_addr)).await {
            Ok(Ok(tcp_stream)) => tcp_stream,
            Ok(Err(source)) => {
                return Err(MagnetMetadataError::Connection { addr: socket_addr, source });
            }
            Err(_) => {
                return Err(MagnetMetadataError::ConnectionTimeout(socket_addr));
            }
        };
        let mut stream = Framed::new(tcp_stream, PeerMessageCodec);

        stream.send(vec![Message::Handshake(Handshake::from_info_hash(info_hash))]).await?;
        Self::read_peer_handshake(&mut stream, info_hash).await?;
        stream.send(vec![PeerExtensionHandshake::local_message()?]).await?;

        let handshake = Self::read_extension_handshake(&mut stream).await?;
        let mut collector = MetadataCollector::new(handshake.metadata_size);
        for piece in 0..collector.piece_count() {
            stream
                .send(vec![MetadataPieceMessage::request(handshake.ut_metadata_id, piece)?])
                .await?;
        }

        while !collector.is_complete() {
            match Self::next_message(&mut stream).await? {
                Message::Extended(message) if message.extension_id == handshake.ut_metadata_id => {
                    let piece = MetadataPieceMessage::parse(message.payload)?;
                    collector.insert(piece)?;
                }
                _ => {}
            }
        }

        collector.assemble()
    }

    async fn read_peer_handshake(stream: &mut Framed<TcpStream, PeerMessageCodec>, info_hash: &[u8]) -> Result<(), MagnetMetadataError> {
        match Self::next_message(stream).await? {
            Message::Handshake(handshake) if handshake.info_hash() == info_hash && handshake.supports_extensions() => Ok(()),
            Message::Handshake(handshake) if handshake.info_hash() != info_hash => Err(MagnetMetadataError::InfoHashMismatch),
            Message::Handshake(_) => Err(MagnetMetadataError::ExtensionProtocolUnsupported),
            _ => Err(MagnetMetadataError::MalformedResponse("expected peer handshake")),
        }
    }

    async fn read_extension_handshake(
        stream: &mut Framed<TcpStream, PeerMessageCodec>,
    ) -> Result<PeerExtensionHandshake, MagnetMetadataError> {
        loop {
            match Self::next_message(stream).await? {
                Message::Extended(message) if message.extension_id == 0 => {
                    return PeerExtensionHandshake::from_payload(&message.payload);
                }
                _ => {}
            }
        }
    }

    async fn next_message(stream: &mut Framed<TcpStream, PeerMessageCodec>) -> Result<Message, MagnetMetadataError> {
        timeout(METADATA_MESSAGE_TIMEOUT, stream.next())
            .await
            .map_err(|_| MagnetMetadataError::PeerTimeout)?
            .ok_or(MagnetMetadataError::PeerClosed)?
            .map_err(MagnetMetadataError::from)
    }
}

#[derive(Debug)]
struct PeerExtensionHandshake {
    ut_metadata_id: u8,
    metadata_size: usize,
}

impl PeerExtensionHandshake {
    fn local_message() -> Result<Message, MagnetMetadataError> {
        let payload = LocalExtensionHandshake::new().encode()?;
        Ok(Message::Extended(ExtendedMessage::new(0, payload)))
    }

    fn from_payload(payload: &[u8]) -> Result<Self, MagnetMetadataError> {
        let decoded: RemoteExtensionHandshake = serde_bencode::de::from_bytes(payload)?;
        let ut_metadata_id = decoded
            .m
            .get("ut_metadata")
            .copied()
            .ok_or(MagnetMetadataError::UtMetadataUnsupported)?;
        let metadata_size = decoded.metadata_size.ok_or(MagnetMetadataError::MetadataSizeMissing)?;
        Ok(Self {
            ut_metadata_id,
            metadata_size,
        })
    }
}

#[derive(Debug, Serialize)]
struct LocalExtensionHandshake {
    m: BTreeMap<String, u8>,
}

impl LocalExtensionHandshake {
    fn new() -> Self {
        Self {
            m: BTreeMap::from([("ut_metadata".to_string(), LOCAL_UT_METADATA_ID)]),
        }
    }

    fn encode(&self) -> Result<Vec<u8>, MagnetMetadataError> {
        Ok(serde_bencode::ser::to_bytes(self)?)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RemoteExtensionHandshake {
    m: BTreeMap<String, u8>,
    #[serde(default)]
    metadata_size: Option<usize>,
}

impl RemoteExtensionHandshake {
    fn new(ut_metadata_id: u8, metadata_size: usize) -> Self {
        Self {
            m: BTreeMap::from([("ut_metadata".to_string(), ut_metadata_id)]),
            metadata_size: Some(metadata_size),
        }
    }

    fn encode(&self) -> Result<Vec<u8>, MagnetMetadataError> {
        Ok(serde_bencode::ser::to_bytes(self)?)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct MetadataPayloadHeader {
    msg_type: u8,
    piece: usize,
    #[serde(default)]
    total_size: Option<usize>,
}

#[derive(Debug)]
struct MetadataPieceMessage {
    piece: usize,
    total_size: Option<usize>,
    data: Vec<u8>,
}

impl MetadataPieceMessage {
    fn request(ut_metadata_id: u8, piece: usize) -> Result<Message, MagnetMetadataError> {
        let header = MetadataPayloadHeader {
            msg_type: 0,
            piece,
            total_size: None,
        };
        Ok(Message::Extended(ExtendedMessage::new(
            ut_metadata_id,
            serde_bencode::ser::to_bytes(&header)?,
        )))
    }

    fn data(ut_metadata_id: u8, piece: usize, total_size: usize, data: Vec<u8>) -> Result<Message, MagnetMetadataError> {
        let header = MetadataPayloadHeader {
            msg_type: 1,
            piece,
            total_size: Some(total_size),
        };
        let mut payload = serde_bencode::ser::to_bytes(&header)?;
        payload.extend(data);
        Ok(Message::Extended(ExtendedMessage::new(ut_metadata_id, payload)))
    }

    fn parse(payload: Vec<u8>) -> Result<Self, MagnetMetadataError> {
        let header_length = BencodePrefix::value_length(&payload)?;
        let header: MetadataPayloadHeader = serde_bencode::de::from_bytes(&payload[..header_length])?;
        match header.msg_type {
            1 => Ok(Self {
                piece: header.piece,
                total_size: header.total_size,
                data: payload[header_length..].to_vec(),
            }),
            2 => Err(MagnetMetadataError::PieceRejected(header.piece)),
            _ => Err(MagnetMetadataError::MalformedResponse("unknown metadata msg_type")),
        }
    }
}

struct MetadataCollector {
    total_size: usize,
    pieces: Vec<Option<Vec<u8>>>,
}

impl MetadataCollector {
    fn new(total_size: usize) -> Self {
        let piece_count = total_size.div_ceil(METADATA_BLOCK_SIZE).max(1);
        Self {
            total_size,
            pieces: vec![None; piece_count],
        }
    }

    fn piece_count(&self) -> usize {
        self.pieces.len()
    }

    fn is_complete(&self) -> bool {
        self.pieces.iter().all(Option::is_some)
    }

    fn insert(&mut self, piece: MetadataPieceMessage) -> Result<(), MagnetMetadataError> {
        if piece.piece >= self.pieces.len() {
            return Err(MagnetMetadataError::PieceOutOfRange {
                piece: piece.piece,
                piece_count: self.pieces.len(),
            });
        }
        if let Some(total_size) = piece.total_size {
            if total_size != self.total_size {
                return Err(MagnetMetadataError::MalformedResponse("metadata total_size changed"));
            }
        }
        self.pieces[piece.piece] = Some(piece.data);
        Ok(())
    }

    fn assemble(self) -> Result<Vec<u8>, MagnetMetadataError> {
        let mut metadata = Vec::with_capacity(self.total_size);
        for piece in self.pieces {
            metadata.extend(piece.ok_or(MagnetMetadataError::MalformedResponse("missing metadata piece"))?);
        }
        metadata.truncate(self.total_size);
        Ok(metadata)
    }
}

struct BencodePrefix;

impl BencodePrefix {
    fn value_length(bytes: &[u8]) -> Result<usize, MagnetMetadataError> {
        Self::scan_value(bytes, 0)
    }

    fn scan_value(bytes: &[u8], offset: usize) -> Result<usize, MagnetMetadataError> {
        let Some(byte) = bytes.get(offset).copied() else {
            return Err(MagnetMetadataError::MalformedResponse("empty bencode value"));
        };

        match byte {
            b'd' | b'l' => Self::scan_list_like(bytes, offset),
            b'i' => Self::scan_integer(bytes, offset),
            b'0'..=b'9' => Self::scan_bytes(bytes, offset),
            _ => Err(MagnetMetadataError::MalformedResponse("invalid bencode prefix")),
        }
    }

    fn scan_list_like(bytes: &[u8], offset: usize) -> Result<usize, MagnetMetadataError> {
        let mut cursor = offset + 1;
        while bytes.get(cursor) != Some(&b'e') {
            cursor = Self::scan_value(bytes, cursor)?;
        }
        Ok(cursor + 1)
    }

    fn scan_integer(bytes: &[u8], offset: usize) -> Result<usize, MagnetMetadataError> {
        bytes[offset..]
            .iter()
            .position(|byte| *byte == b'e')
            .map(|position| offset + position + 1)
            .ok_or(MagnetMetadataError::MalformedResponse("unterminated bencode integer"))
    }

    fn scan_bytes(bytes: &[u8], offset: usize) -> Result<usize, MagnetMetadataError> {
        let colon_offset = bytes[offset..]
            .iter()
            .position(|byte| *byte == b':')
            .ok_or(MagnetMetadataError::MalformedResponse("unterminated bencode byte string"))?
            + offset;
        let length = std::str::from_utf8(&bytes[offset..colon_offset])
            .ok()
            .and_then(|digits| digits.parse::<usize>().ok())
            .ok_or(MagnetMetadataError::MalformedResponse("invalid bencode byte string length"))?;
        let end = colon_offset + 1 + length;
        if end > bytes.len() {
            return Err(MagnetMetadataError::MalformedResponse("truncated bencode byte string"));
        }
        Ok(end)
    }
}

#[cfg(test)]
mod tests {
    use super::{MagnetMetadataFetcher, MetadataPieceMessage, RemoteExtensionHandshake, METADATA_BLOCK_SIZE};
    use crate::core::peer::{
        messages::{Handshake, Message},
        PeerMessageCodec,
    };
    use futures_util::{SinkExt, StreamExt};
    use hyperblow::parser::torrent_parser::Info;
    use sha1::{Digest, Sha1};
    use tokio::net::TcpListener;
    use tokio_util::codec::Framed;

    #[tokio::test]
    async fn fetches_metadata_from_extension_peer() {
        let metadata = MetadataFixture::single_piece();
        let info_hash = MetadataFixture::info_hash(&metadata);
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener should bind");
        let address = listener.local_addr().expect("listener should have address");
        let server = tokio::spawn(MetadataPeerFixture::serve(listener, info_hash.clone(), metadata.clone()));

        let fetched = MagnetMetadataFetcher::fetch(address, &info_hash)
            .await
            .expect("metadata should fetch");

        assert_eq!(fetched, metadata);
        server.await.expect("server should finish");
    }

    #[test]
    fn splits_metadata_response_header_from_payload() {
        let block = vec![1, 2, 3, 4];
        let message = match MetadataPieceMessage::data(3, 0, 4, block.clone()).expect("message should encode") {
            Message::Extended(message) => message,
            _ => unreachable!("metadata message is extended"),
        };

        let parsed = MetadataPieceMessage::parse(message.payload).expect("payload should parse");

        assert_eq!(parsed.piece, 0);
        assert_eq!(parsed.total_size, Some(4));
        assert_eq!(parsed.data, block);
    }

    struct MetadataFixture;

    impl MetadataFixture {
        fn single_piece() -> Vec<u8> {
            let info = Info {
                name: Some("metadata-test.bin".to_string()),
                length: Some(4),
                files: None,
                piece_length: Some(4),
                pieces: vec![0; 20],
            };
            let metadata = serde_bencode::ser::to_bytes(&info).expect("info should encode");
            assert!(metadata.len() < METADATA_BLOCK_SIZE);
            metadata
        }

        fn info_hash(metadata: &[u8]) -> Vec<u8> {
            Sha1::digest(metadata).to_vec()
        }
    }

    struct MetadataPeerFixture;

    impl MetadataPeerFixture {
        async fn serve(listener: TcpListener, info_hash: Vec<u8>, metadata: Vec<u8>) {
            let (socket, _) = listener.accept().await.expect("client should connect");
            let mut stream = Framed::new(socket, PeerMessageCodec);

            match stream.next().await.expect("handshake frame").expect("handshake decode") {
                Message::Handshake(handshake) => {
                    assert_eq!(handshake.info_hash(), info_hash.as_slice());
                    assert!(handshake.supports_extensions());
                }
                message => panic!("expected handshake, got {message:?}"),
            }

            stream
                .send(vec![Message::Handshake(Handshake::from_info_hash(&info_hash))])
                .await
                .expect("server handshake should send");
            match stream.next().await.expect("extension handshake frame").expect("extension decode") {
                Message::Extended(message) if message.extension_id == 0 => {
                    let handshake: RemoteExtensionHandshake =
                        serde_bencode::de::from_bytes(&message.payload).expect("client extension handshake should parse");
                    assert_eq!(handshake.m.get("ut_metadata"), Some(&1));
                }
                message => panic!("expected extension handshake, got {message:?}"),
            }

            stream
                .send(vec![Message::Extended(super::ExtendedMessage::new(
                    0,
                    RemoteExtensionHandshake::new(3, metadata.len())
                        .encode()
                        .expect("server extension handshake should encode"),
                ))])
                .await
                .expect("server extension handshake should send");

            let piece = match stream.next().await.expect("metadata request frame").expect("request decode") {
                Message::Extended(message) if message.extension_id == 3 => {
                    let request = MetadataPieceMessage::parse_request_for_test(message.payload);
                    request.piece
                }
                message => panic!("expected metadata request, got {message:?}"),
            };
            assert_eq!(piece, 0);

            stream
                .send(vec![
                    MetadataPieceMessage::data(3, 0, metadata.len(), metadata).expect("metadata response should encode")
                ])
                .await
                .expect("metadata response should send");
        }
    }

    impl MetadataPieceMessage {
        fn parse_request_for_test(payload: Vec<u8>) -> Self {
            let header: super::MetadataPayloadHeader = serde_bencode::de::from_bytes(&payload).expect("request should parse");
            assert_eq!(header.msg_type, 0);
            Self {
                piece: header.piece,
                total_size: None,
                data: Vec::new(),
            }
        }
    }
}
