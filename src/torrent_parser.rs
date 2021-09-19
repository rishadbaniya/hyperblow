use std::fs;

/// DataStructure that maps all the data inside of bencode
/// encoded .torrent file into something rust program can use
///
/// Option<T> allows me to give None when the field is not present
///
/// #serde[rename] attribute lets me deserialize from the given name in that attribute
/// Eg. if the name in the field inside of torrent file is "your name", which cannot be possibly used inside a Struct
/// then we can say
///
/// {
/// #[serde(alias = "your name")]
/// your_name : String
/// }
///
/// which means "hey, the value of the field that has name "your name" in the
/// torrent file, map its value to the struct field below"
///

#[derive(Debug, Deserialize)]
pub struct FileMeta {
    announce: Option<String>,
    #[serde(rename = "announce-list")]
    announce_list: Option<Vec<Vec<String>>>,
    #[serde(rename = "")]
    creation_data: Option<i64>,
}

pub fn parse(filePath: &str) -> FileMeta {
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
    decoded
}
