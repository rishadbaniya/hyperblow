#![allow(non_snake_case, dead_code)]

use magnet_url::Magnet;
use std::{error, fmt};

#[derive(Debug,)]
pub enum MagnetURIMetaError {
    InvalidURI,
}

impl fmt::Display for MagnetURIMetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_,>,) -> fmt::Result {
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
struct MagnetURIMeta {
    /// **(Required)** Exact Topic : Info Hash of the torrent and the type of hash being used is
    /// also kept here
    pub xt: Option<String,>,

    /// **(Optional)** Display name : The filename to display to the user
    pub dn: Option<String,>,

    /// **(Optional)** Exact Length : The size of the file in bytes
    pub xl: Option<u64,>,

    /// **(Optional)** Address Tracker : The url of the tracker
    pub tr: Option<Vec<String,>,>,

    /// **(Optional)** Web Seed : They payload data served over HTTP(S)
    pub ws: Option<String,>,

    /// **(Optional)** As "as" is a reserved keyword in rust, acceptable_source as in whole word is
    /// written, which Refers to a direct download from a web server. It's URL encoded
    pub acceptable_source: Option<String,>,

    /// **(Optional)** eXact Source: Either an HTTP (or HTTPS, FTP, FTPS, etc.) download source for the file pointed
    /// to by the Magnet link, the address of a P2P source for the file or the address of a hub (in
    /// the case of DC++), by which a client tries to connect directly, asking for the file and/or
    /// its sources. This field is commonly used by P2P clients to store the source, and may include
    /// the file hash.
    pub xs: Option<String,>,

    /// **(Optional)** Specifies a string of search keywords to search for in P2P networks, rather than a particular file
    ///kt=martin+luther+king+mp3   
    pub kt: Option<String,>,

    /// **(Optional)** Manifest Topic : Link to the metafile that contains a list of magneto (MAGMA – MAGnet MAnifest)
    pub mt: Option<String,>,
}

impl MagnetURIMeta {
    /// Tries to create [MagnetURIMeta] from given magnet URI
    fn fromMagnetURI(uri: &String,) -> Result<MagnetURIMeta, MagnetURIMetaError,> {
        return match Magnet::new(uri,) {
            Ok(d,) => Ok(MagnetURIMeta {
                xt: d.xt,
                dn: d.dn,
                xl: d.xl,
                tr: Some(d.tr,),
                ws: d.ws,
                xs: d.xs,
                kt: d.kt,
                mt: d.mt,
                acceptable_source: d.acceptable_source,
            },),
            Err(_,) => Err(MagnetURIMetaError::InvalidURI,),
        };
    }

    /// Checks if the Magnet URI is valid or not
    fn checkIfMagnetURIIsValid(uri: &String,) -> bool {
        return match Magnet::new(uri,) {
            Ok(_,) => true,
            Err(_,) => false,
        };
    }
}
