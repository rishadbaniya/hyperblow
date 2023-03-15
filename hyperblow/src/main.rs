#![allow(non_snake_case, dead_code)]

mod arguments;
mod core;
mod engine;
mod ui;

use arguments::Arguments;
use clap::Parser;
use engine::Engine;
use engine::TorrentSource;

//use std::{env, error::Error, sync::Arc, thread, time::Instant};
//use tokio::sync::Mutex;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Arguments::parse();
    args.check();
    println!("{args:?}");

    let engine = Engine::new();
    engine.spawn(TorrentSource::FilePath(args.torrent_file.unwrap())).await;

    ui::ui::draw_ui(engine.clone()).await?;

    Ok(())
}
