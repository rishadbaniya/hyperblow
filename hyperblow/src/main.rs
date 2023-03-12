#![allow(non_snake_case)]

mod core;
mod engine;

use clap::Parser;
use engine::Engine;

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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Arguments::parse();
    let mut engine = Engine::new();

    //if let Some(ref path) = args.torrent_file {
    //    //    let path = path.clone();
    //    //    //        let torrent_handle = engine.spawn(TorrentSource::FilePath(path)).await;
    //}
}
