#![allow(non_snake_case, dead_code)]

use magnet_url::Magnet;
use std::{error, fmt};

#[derive(Debug)]
pub enum MagnetURIMetaError {
    InvalidURI,
}

impl fmt::Display for MagnetURIMetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MagnetURIMetaError::InvalidURI => write!(f, "Invalid magnet URI"),
        }
    }
}

impl error::Error for MagnetURIMetaError {}

/// DataStructure that maps all the data withing a Magnet URI into something rust program can use.
///
/// The fields of MagnetURIMeta were taken from Wikipedia : "https://en.wikipedia.org/wiki/Magnet_URI_scheme"
///
/// Some of the fields from the crate "magnet_url" itself
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagnetURIMeta {
    /// **(Required)** Exact Topic : Info Hash of the torrent and the type of hash being used is
    /// also kept here
    pub xt: Option<String>,

    /// **(Optional)** Display name : The filename to display to the user
    pub dn: Option<String>,

    /// **(Optional)** Exact Length : The size of the file in bytes
    pub xl: Option<u64>,

    /// **(Optional)** Address Tracker : The url of the tracker
    pub tr: Option<Vec<String>>,

    /// **(Optional)** Web Seed : They payload data served over HTTP(S)
    pub ws: Option<String>,

    /// **(Optional)** As "as" is a reserved keyword in rust, acceptable_source as in whole word is
    /// written, which Refers to a direct download from a web server. It's URL encoded
    pub acceptable_source: Option<String>,

    /// **(Optional)** eXact Source: Either an HTTP (or HTTPS, FTP, FTPS, etc.) download source for the file pointed
    /// to by the Magnet link, the address of a P2P source for the file or the address of a hub (in
    /// the case of DC++), by which a client tries to connect directly, asking for the file and/or
    /// its sources. This field is commonly used by P2P clients to store the source, and may include
    /// the file hash.
    pub xs: Option<String>,

    /// **(Optional)** Specifies a string of search keywords to search for in P2P networks, rather than a particular file
    ///kt=martin+luther+king+mp3   
    pub kt: Option<String>,

    /// **(Optional)** Manifest Topic : Link to the metafile that contains a list of magneto (MAGMA – MAGnet MAnifest)
    pub mt: Option<String>,
}

impl MagnetURIMeta {
    /// Tries to create [MagnetURIMeta] from given magnet URI
    pub fn fromMagnetURI(uri: &str) -> Result<MagnetURIMeta, MagnetURIMetaError> {
        match Magnet::new(uri) {
            Ok(d) => {
                let xt = match (d.hash_type(), d.hash()) {
                    (Some(hash_type), Some(hash)) => Some(format!("urn:{hash_type}:{hash}")),
                    _ => None,
                };

                Ok(MagnetURIMeta {
                    xt,
                    dn: d.display_name().map(ToOwned::to_owned),
                    xl: d.length(),
                    tr: Some(d.trackers().to_vec()),
                    ws: d.web_seed().map(ToOwned::to_owned),
                    xs: d.source().map(ToOwned::to_owned),
                    kt: d.search_keywords().map(ToOwned::to_owned),
                    mt: d.manifest().map(ToOwned::to_owned),
                    acceptable_source: d.acceptable_source().map(ToOwned::to_owned),
                })
            }
            Err(_) => Err(MagnetURIMetaError::InvalidURI),
        }
    }

    /// Checks if the Magnet URI is valid or not
    pub fn checkIfMagnetURIIsValid(uri: &str) -> bool {
        Magnet::new(uri).is_ok()
    }
}
