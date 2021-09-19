#![allow(non_snake_case)]

extern crate serde;
extern crate serde_bencode;
#[macro_use]
extern crate serde_derive;

mod torrent_parser;

use std::env;
use torrent_parser::FileMeta;

fn main() {
    // Get all arguments passed in the CLI
    let args: Vec<String> = env::args().collect();

    // Get the argument at index 1 from the CLI command "rtourent xyz.torrent"
    // So that we can get the name of the file i.e xyz.torrent
    let torrentParsed: FileMeta = torrent_parser::parse(&args[1]);

    println!("{:?}", torrentParsed);
}
