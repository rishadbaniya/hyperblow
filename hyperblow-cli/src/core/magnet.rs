use super::{state::State, TError, TorrentFile};
use hyperblow::parser::{
    magnet_uri_parser::MagnetURIMeta,
    torrent_parser::{FileMeta, Info},
};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info};

#[derive(Debug, Error)]
pub enum MagnetTorrentError {
    #[error("magnet URI is missing a BTIH exact topic")]
    MissingBtih,

    #[error("unsupported BTIH hash: expected 40 hex or 32 base32 characters, got {len}")]
    UnsupportedBtihLength { len: usize },

    #[error("invalid BTIH hash character")]
    InvalidBtihCharacter,

    #[error("magnet tracker session could not be created")]
    Session(#[from] TError),
}

#[derive(Debug)]
pub struct MagnetTorrent {
    meta: Arc<MagnetURIMeta>,
    session: TorrentFile,
    info_hash: [u8; 20],
}

impl MagnetTorrent {
    pub async fn new(meta: MagnetURIMeta) -> Result<Self, MagnetTorrentError> {
        let info_hash = MagnetInfoHash::from_meta(&meta)?;
        let tracker_count = meta.tr.as_ref().map_or(0, Vec::len);
        let file_meta = MagnetFileMeta::from_magnet(&meta);
        let session = TorrentFile::from_metadata_with_info_hash("magnet".to_string(), file_meta, info_hash.to_vec(), false).await?;
        info!(tracker_count, "created magnet tracker session");

        Ok(Self {
            meta: Arc::new(meta),
            session,
            info_hash,
        })
    }

    pub async fn run(&self) {
        debug!(tracker_count = self.tracker_addresses().len(), "running magnet tracker session");
        self.session.run().await;
    }

    pub fn meta(&self) -> &MagnetURIMeta {
        &self.meta
    }

    pub fn state(&self) -> Arc<State> {
        self.session.state.clone()
    }

    pub fn info_hash(&self) -> &[u8; 20] {
        &self.info_hash
    }

    pub fn bytes_total(&self) -> Option<usize> {
        self.meta.xl.map(|size| size as usize)
    }

    pub fn tracker_addresses(&self) -> Vec<String> {
        self.meta.tr.clone().unwrap_or_default()
    }

    pub fn status_label(&self) -> String {
        if self.tracker_addresses().is_empty() {
            "No magnet trackers".to_string()
        } else {
            "Fetching metadata".to_string()
        }
    }
}

struct MagnetFileMeta;

impl MagnetFileMeta {
    fn from_magnet(meta: &MagnetURIMeta) -> FileMeta {
        let trackers = meta.tr.clone().unwrap_or_default();
        FileMeta {
            announce: trackers.first().cloned().unwrap_or_default(),
            announce_list: (!trackers.is_empty()).then_some(vec![trackers]),
            info: Info {
                name: meta.dn.clone(),
                length: meta.xl.map(|length| length as i64),
                files: None,
                piece_length: None,
                pieces: Vec::new(),
            },
            creation_data: None,
            comment: Some("magnet metadata pending".to_string()),
            encoding: None,
            created_by: None,
            acceptable_source: None,
        }
    }
}

struct MagnetInfoHash;

impl MagnetInfoHash {
    fn from_meta(meta: &MagnetURIMeta) -> Result<[u8; 20], MagnetTorrentError> {
        let exact_topic = meta.xt.as_deref().ok_or(MagnetTorrentError::MissingBtih)?;
        let hash = exact_topic
            .strip_prefix("urn:btih:")
            .or_else(|| exact_topic.strip_prefix("btih:"))
            .ok_or(MagnetTorrentError::MissingBtih)?;
        Self::decode(hash)
    }

    fn decode(hash: &str) -> Result<[u8; 20], MagnetTorrentError> {
        match hash.len() {
            40 => HexBtih::decode(hash),
            32 => Base32Btih::decode(hash),
            len => Err(MagnetTorrentError::UnsupportedBtihLength { len }),
        }
    }
}

struct HexBtih;

impl HexBtih {
    fn decode(hash: &str) -> Result<[u8; 20], MagnetTorrentError> {
        let mut decoded = [0_u8; 20];
        for (index, chunk) in hash.as_bytes().chunks_exact(2).enumerate() {
            let high = Self::hex_value(chunk[0])?;
            let low = Self::hex_value(chunk[1])?;
            decoded[index] = (high << 4) | low;
        }
        Ok(decoded)
    }

    fn hex_value(byte: u8) -> Result<u8, MagnetTorrentError> {
        match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            b'A'..=b'F' => Ok(byte - b'A' + 10),
            _ => Err(MagnetTorrentError::InvalidBtihCharacter),
        }
    }
}

struct Base32Btih;

impl Base32Btih {
    fn decode(hash: &str) -> Result<[u8; 20], MagnetTorrentError> {
        let mut bytes = Vec::with_capacity(20);
        let mut buffer = 0_u32;
        let mut bits = 0_u8;

        for byte in hash.bytes() {
            let value = Self::base32_value(byte)? as u32;
            buffer = (buffer << 5) | value;
            bits += 5;

            while bits >= 8 {
                bits -= 8;
                bytes.push((buffer >> bits) as u8);
                buffer &= (1 << bits) - 1;
            }
        }

        bytes
            .try_into()
            .map_err(|bytes: Vec<u8>| MagnetTorrentError::UnsupportedBtihLength { len: bytes.len() })
    }

    fn base32_value(byte: u8) -> Result<u8, MagnetTorrentError> {
        match byte {
            b'A'..=b'Z' => Ok(byte - b'A'),
            b'a'..=b'z' => Ok(byte - b'a'),
            b'2'..=b'7' => Ok(byte - b'2' + 26),
            _ => Err(MagnetTorrentError::InvalidBtihCharacter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MagnetInfoHash, MagnetTorrent};
    use hyperblow::parser::magnet_uri_parser::MagnetURIMeta;

    #[test]
    fn decodes_uppercase_hex_btih() {
        let decoded = MagnetInfoHash::decode("19F25D256632318E48B4E814A26D257BC3E213A9").expect("hex BTIH should decode");

        assert_eq!(&decoded[..4], &[0x19, 0xF2, 0x5D, 0x25]);
        assert_eq!(decoded.len(), 20);
    }

    #[tokio::test]
    async fn creates_tracker_session_from_magnet_trackers() {
        let meta = MagnetURIMeta::fromMagnetURI(
            "magnet:?xt=urn:btih:19F25D256632318E48B4E814A26D257BC3E213A9&dn=Example&tr=udp%3A%2F%2Ftracker.example.com%3A6969%2Fannounce",
        )
        .expect("magnet should parse");

        let torrent = MagnetTorrent::new(meta).await.expect("magnet torrent should initialize");

        assert_eq!(&torrent.info_hash()[..4], &[0x19, 0xF2, 0x5D, 0x25]);
        assert_eq!(torrent.bytes_total(), None);
        assert_eq!(torrent.tracker_addresses(), vec!["udp://tracker.example.com:6969/announce"]);
        assert_eq!(torrent.status_label(), "Fetching metadata");
    }
}
