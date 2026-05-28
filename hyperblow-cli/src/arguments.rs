use std::path::Path;

use clap::Parser;
use hyperblow::parser::magnet_uri_parser::MagnetURIMeta;
use thiserror::Error;

#[derive(Debug, Parser, Default)]
#[clap(author = "Rishad Baniya", version)]
pub struct Arguments {
    /// Path to the torrent file you wish to download
    #[arg(short('f'), long("file"), value_name = "TORRENT")]
    pub torrent_file: Option<String>,

    /// URI of the torrent file you wish to download
    #[arg(short('m'), long("magnet"), value_name = "URI")]
    pub magnet_uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TorrentInput {
    FilePath(String),
    MagnetUri(String),
}

#[derive(Debug, Error)]
pub enum ArgumentError {
    #[error("provide either a torrent file or a magnet URI, not both")]
    MultipleSources,

    #[error("torrent file does not exist or is not a file: {0}")]
    InvalidTorrentFile(String),

    #[error("invalid magnet URI")]
    InvalidMagnetUri,
}

impl Arguments {
    /// Checks if the torrent_file argument provided or not, doesn't validate by checking
    /// if the file exists, or is a valid bencode encoded torrent file or not
    pub fn is_file_argument_provided(&self) -> bool {
        self.torrent_file.is_some()
    }

    /// Checks if the magnet_uri argument provided or not, doesn't validate by checking
    /// if the magnet uri is valid or not
    pub fn is_magnet_uri_provided(&self) -> bool {
        self.magnet_uri.is_some()
    }

    /// Checks if both arguments are provided
    pub fn is_both_argument_provided(&self) -> bool {
        self.is_file_argument_provided() && self.is_magnet_uri_provided()
    }

    /// Checks if none of the arguments are provided
    pub fn none_arguments_provided(&self) -> bool {
        !(self.is_file_argument_provided() || self.is_magnet_uri_provided())
    }

    pub fn source(&self) -> Result<Option<TorrentInput>, ArgumentError> {
        if self.is_both_argument_provided() {
            return Err(ArgumentError::MultipleSources);
        }

        if let Some(path) = self.torrent_file.as_ref() {
            if !Path::new(path).is_file() {
                return Err(ArgumentError::InvalidTorrentFile(path.clone()));
            }
            return Ok(Some(TorrentInput::FilePath(path.clone())));
        }

        if let Some(uri) = self.magnet_uri.as_ref() {
            if !MagnetURIMeta::checkIfMagnetURIIsValid(uri) {
                return Err(ArgumentError::InvalidMagnetUri);
            }
            return Ok(Some(TorrentInput::MagnetUri(uri.clone())));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::{ArgumentError, Arguments, TorrentInput};

    #[test]
    fn no_source_starts_idle() {
        let args = Arguments::default();

        assert_eq!(args.source().expect("no source should be valid"), None);
    }

    #[test]
    fn rejects_file_and_magnet_together() {
        let args = Arguments {
            torrent_file: Some("test.torrent".to_string()),
            magnet_uri: Some("magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10".to_string()),
        };

        assert!(matches!(args.source(), Err(ArgumentError::MultipleSources)));
    }

    #[test]
    fn accepts_valid_magnet_source() {
        let uri = "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10".to_string();
        let args = Arguments {
            torrent_file: None,
            magnet_uri: Some(uri.clone()),
        };

        assert_eq!(args.source().expect("magnet should be valid"), Some(TorrentInput::MagnetUri(uri)));
    }
}
