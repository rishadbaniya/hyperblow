#![allow(non_snake_case)]

mod core;

use crate::core::TorrentFile;
use clap::Parser;

//use std::{env, error::Error, sync::Arc, thread, time::Instant};
//use tokio::sync::Mutex;

#[derive(Debug, Parser, Default)]
#[clap(author = "Rishad Baniya", version)]
struct Arguments {
    #[arg(short('f'))]
    /// Path to the torrent file you wish to download
    torrent_file: Option<String>,

    /// URI of the torrent file you wish to download
    #[arg(short('u'))]
    magnet_uri: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    if let Some(ref path) = args.torrent_file {
        let x = TorrentFile::new(path);
        //        if let Some(d) = x {
        //            println!("{:#?}", d.fileTree);
        //            d.resolveTrackers();
        //            println!("{:#?}", d.trackers);
        //        }
    }
}
