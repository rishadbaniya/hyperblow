#![allow(non_snake_case)]
extern crate serde;
extern crate serde_bencode;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::fs;

// DataStructure that maps all the data inside of bencode
// encoded .torrent file into something rust program can use
#[derive(Debug, Deserialize)]
struct FileMeta {
    announce: String,
    announce_list: Vec<String>,
}

fn main() {
    // Declared to store all bytes from torrent file
    let _torrentFile: Vec<u8>;

    // Get all arguments passed
    let args: Vec<String> = env::args().collect();

    // Get the argument at index 1 from the CLI command "rtourent xyz.torrent"
    // So that we can get the name of the file i.e xyz.torrent
    if let Ok(data) = fs::read(&args[1]) {
        // Store the all bytes in _torrentFile
        _torrentFile = data
    } else {
        // If there is no file availaible of that name then exit the program
        println!("Sorry, i could not find a file named \"{}\"", &args[1]);
        std::process::exit(0);
    }

    // Decode the bencode format into Rust Custom DataStructure "FileMeta"
    let decoded: FileMeta = serde_bencode::de::from_bytes(&_torrentFile).unwrap();
}
