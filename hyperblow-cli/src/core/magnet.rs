use super::{state::State, TError, TorrentFile};
use hyperblow::parser::{
    magnet_uri_parser::MagnetURIMeta,
    torrent_parser::{FileMeta, Info},
};
use sha1::{Digest, Sha1};
use std::{path::PathBuf, sync::Arc};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

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

    #[error("metadata info hash did not match magnet BTIH")]
    MetadataHashMismatch,

    #[error("metadata bencode could not be decoded")]
    MetadataBencode(#[from] serde_bencode::Error),
}

#[derive(Debug)]
pub struct MagnetTorrent {
    meta: Arc<MagnetURIMeta>,
    session: TorrentFile,
    info_hash: [u8; 20],
    resolved: Arc<RwLock<Option<Arc<TorrentFile>>>>,
    download_directory: PathBuf,
}

impl MagnetTorrent {
    pub async fn new(meta: MagnetURIMeta, download_directory: PathBuf) -> Result<Self, MagnetTorrentError> {
        let info_hash = MagnetInfoHash::from_meta(&meta)?;
        let tracker_count = meta.tr.as_ref().map_or(0, Vec::len);
        let file_meta = MagnetFileMeta::from_magnet(&meta);
        let session = TorrentFile::from_metadata_with_info_hash(
            "magnet".to_string(),
            file_meta,
            info_hash.to_vec(),
            false,
            download_directory.clone(),
        )
        .await?;
        info!(tracker_count, "created magnet tracker session");

        Ok(Self {
            meta: Arc::new(meta),
            session,
            info_hash,
            resolved: Arc::new(RwLock::new(None)),
            download_directory,
        })
    }

    pub async fn run(&self) {
        debug!(tracker_count = self.tracker_addresses().len(), "running magnet metadata session");
        match self.resolve_metadata().await {
            Ok(torrent) => {
                info!(torrent = %self.display_name(), "magnet metadata fetched");
                *self.resolved.write().await = Some(torrent.clone());
                torrent.run().await;
            }
            Err(error) => {
                warn!(error = %error, "magnet metadata fetch failed");
            }
        }
    }

    pub fn meta(&self) -> &MagnetURIMeta {
        &self.meta
    }

    pub fn state(&self) -> Arc<State> {
        if let Ok(resolved) = self.resolved.try_read() {
            if let Some(torrent) = resolved.as_ref() {
                return torrent.state.clone();
            }
        }
        self.session.state.clone()
    }

    pub fn info_hash(&self) -> &[u8; 20] {
        &self.info_hash
    }

    pub fn bytes_total(&self) -> Option<usize> {
        if let Ok(resolved) = self.resolved.try_read() {
            if let Some(torrent) = resolved.as_ref() {
                return Some(torrent.state.meta_info.total_length().max(0) as usize);
            }
        }
        self.meta.xl.map(|size| size as usize)
    }

    pub fn tracker_addresses(&self) -> Vec<String> {
        self.meta.tr.clone().unwrap_or_default()
    }

    pub fn status_label(&self) -> String {
        if self.tracker_addresses().is_empty() {
            "No magnet trackers".to_string()
        } else if self
            .resolved
            .try_read()
            .ok()
            .and_then(|resolved| resolved.as_ref().map(|_| ()))
            .is_some()
        {
            "Metadata fetched".to_string()
        } else {
            "Fetching metadata".to_string()
        }
    }

    async fn resolve_metadata(&self) -> Result<Arc<TorrentFile>, MagnetTorrentError> {
        let metadata = self.session.fetch_magnet_metadata().await?;
        self.validate_metadata_hash(&metadata)?;
        let info = serde_bencode::de::from_bytes::<Info>(&metadata)?;
        let file_meta = MagnetFileMeta::from_info(self.meta(), info);
        Ok(Arc::new(
            TorrentFile::from_metadata_with_info_hash(
                "magnet".to_string(),
                file_meta,
                self.info_hash.to_vec(),
                true,
                self.download_directory.clone(),
            )
            .await?,
        ))
    }

    fn validate_metadata_hash(&self, metadata: &[u8]) -> Result<(), MagnetTorrentError> {
        let actual_hash: [u8; 20] = Sha1::digest(metadata).into();
        if actual_hash == self.info_hash {
            Ok(())
        } else {
            Err(MagnetTorrentError::MetadataHashMismatch)
        }
    }

    fn display_name(&self) -> String {
        self.meta.dn.clone().unwrap_or_else(|| "Magnet torrent".to_string())
    }

    #[cfg(test)]
    pub(crate) async fn set_resolved_for_test(&self, torrent: Arc<TorrentFile>) {
        *self.resolved.write().await = Some(torrent);
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

    fn from_info(meta: &MagnetURIMeta, info: Info) -> FileMeta {
        let trackers = meta.tr.clone().unwrap_or_default();
        FileMeta {
            announce: trackers.first().cloned().unwrap_or_default(),
            announce_list: (!trackers.is_empty()).then_some(vec![trackers]),
            info,
            creation_data: None,
            comment: Some("magnet metadata fetched".to_string()),
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
        let decoded = MagnetInfoHash::decode("0123456789ABCDEF0123456789ABCDEF01234567").expect("hex BTIH should decode");

        assert_eq!(&decoded[..4], &[0x01, 0x23, 0x45, 0x67]);
        assert_eq!(decoded.len(), 20);
    }

    #[tokio::test]
    async fn creates_tracker_session_from_magnet_trackers() {
        let meta = MagnetURIMeta::fromMagnetURI(
            "magnet:?xt=urn:btih:0123456789ABCDEF0123456789ABCDEF01234567&dn=Example&tr=udp%3A%2F%2Ftracker.example.com%3A6969%2Fannounce",
        )
        .expect("magnet should parse");

        let torrent = MagnetTorrent::new(meta, std::env::temp_dir())
            .await
            .expect("magnet torrent should initialize");

        assert_eq!(&torrent.info_hash()[..4], &[0x01, 0x23, 0x45, 0x67]);
        assert_eq!(torrent.bytes_total(), None);
        assert_eq!(torrent.tracker_addresses(), vec!["udp://tracker.example.com:6969/announce"]);
        assert_eq!(torrent.status_label(), "Fetching metadata");
    }
}
