use serde_derive::{Deserialize, Serialize};

use sha1::{Digest, Sha1};
use std::fs;

/// DataStructure that maps all the data inside of bencode encoded ".torrent" file into something rust program can use.
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
///
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

pub fn parse_file(filePath: &String) -> (FileMeta, [u8; 20]) {
    // Declared to store all bytes from torrent file
    let torrentFile: Vec<u8>;

    if let Ok(data) = fs::read(filePath) {
        // Store the all bytes in _torrentFile
        torrentFile = data;
    } else {
        // If there is no file availaible of that name then exit the program
        println!("Sorry, i could not find a file named \"{}\"", filePath);
        std::process::exit(0);
    }

    // Decode the bencode format into Rust Custom DataStructure "FileMeta"
    let decoded: FileMeta = serde_bencode::de::from_bytes(&torrentFile).unwrap();

    // Serialize the info section of FileMeta and get all bytes in info field of a torrent file
    let infoByte = serde_bencode::ser::to_bytes(&decoded.info).unwrap();

    // SHA1 hash of the infoByte i.e info_hash
    let info_hash = generateInfoHash(infoByte);

    (decoded, info_hash)
}

// Takes all the byetes that's in the info field and generates a hash out of it,
// called "info hash"
fn generateInfoHash(info_byte: impl AsRef<[u8]>) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(info_byte);
    hasher.finalize().into()
}
