#![allow(non_snake_case, dead_code)]

use magnet_uri::MagnetURI;
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{fs, str::FromStr};

/// TODO : Add a parsing method for "magnet links"
///
/// DataStructure that maps all the data inside of bencode encoded ".torrent" file
/// into something rust program can use.
/// Option<T> allows us to give None when the field is not present
///
/// #serde[rename] attribute lets me deserialize from the given name in that attribute
/// Eg. if the name in the field inside of torrent file is "your name", which cannot be possibly used inside a Struct
/// then we can say
///
/// {
/// #[serde(alias = "your name")]
///     your_name : String
/// }
///
/// which means "hey, the value of the field that has key "your name" in the
/// torrent file, map its value to the struct field below"

#[derive(Debug, Deserialize)]
pub struct FileMeta {
    pub announce: String,
    #[serde(rename = "announce-list")]
    pub announce_list: Option<Vec<Vec<String>>>,
    #[serde(rename = "creation date")]
    pub creation_data: Option<i64>,
    pub encoding: Option<String>,
    pub comment: Option<String>,
    #[serde(rename = "created by")]
    pub created_by: Option<String>,
    pub info: Info,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Info {
    pub name: Option<String>,
    pub length: Option<i64>,
    #[serde(rename = "piece length")]
    pub piece_length: Option<i64>,
    /// Consists of byte string of concatenation of all 20-byte SHA1 hash values, one per piece
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
    pub files: Option<Vec<File>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct File {
    pub length: i64,
    pub path: Vec<String>,
    pub md5sum: Option<String>,
}

impl FileMeta {
    // PARSING TORRENT FILE
    // Just pass in your path to the torrent file, it will return a
    // DataStructure[FileMeta] that contains all the metadata that was within the ".torrent" file
    pub fn parseTorrentFile(file_path: &String) -> FileMeta {
        // Creates a buffer to store the bytes of the file
        let torrent_file: Vec<u8>;

        match fs::read(file_path) {
            Ok(bytes) => {
                // If the file exits, read the file and then store the bytes
                torrent_file = bytes;
            }
            Err(_) => {
                println!("Sorry, could not locate \"{}\"", *file_path);
                std::process::exit(0);
            }
        }

        // Parses the bytes of the ".torrent" file into the Data Structure "FileMeta" i.e
        // Maps the data in the bytes to the data structure FileMeta
        let meta_data: FileMeta = serde_bencode::de::from_bytes(&torrent_file).unwrap();

        return meta_data;
    }

    // TODO : Find a way to parse magnet link
    pub fn parseMagnetLink(magnet_link: &String) -> () {
        // TODO : MAKE IT USABLE | AS OF RIGHT NOW IT"S NOT USABLE
        match MagnetURI::from_str(&magnet_link.as_ref()) {
            Ok(data) => {}
            Err(_) => {
                // Throws you some kind of error when the magnet link isn't valid
                println!("Enter a valid magnet link!");
            }
        }
    }

    // PARSING MAGNET LINK
    // Gets you the Info Hash
    fn get_info_hash(&self) -> [u8; 20] {
        // Serialize the info section of FileMeta and get all bytes in info field of a torrent file
        // i.e Converts the data of info field to "bytes", to generate Info Hash
        let info_byte = serde_bencode::ser::to_bytes(&self.info).unwrap();

        // which is eventually called "Info Hash"
        let mut hasher = Sha1::new();
        hasher.update(info_byte);
        hasher.finalize().into()
    }
}
