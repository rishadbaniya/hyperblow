#![allow(non_snake_case)]

extern crate serde;
extern crate serde_bencode;
#[macro_use]
extern crate serde_derive;

mod percent_encoder;
mod torrent_details;
pub mod torrent_parser;

use std::env;
use torrent_details::spit_details;

fn main() {
    // Get all arguments passed in the CLI
    let args: Vec<String> = env::args().collect();
    // Get the argument at index 1 from the CLI command "rtourent xyz.torrent"
    // So that we can get the name of the file i.e xyz.torrent
    let (torrentParsed, info_hash_bytes) = torrent_parser::parse_file(&args[1]);
    spit_details(&torrentParsed);
    let percentEncodedInfoHash = percent_encoder::encode(info_hash_bytes);
    println!("{}", percentEncodedInfoHash);
}
