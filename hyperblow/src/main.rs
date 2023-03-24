#![allow(non_snake_case, dead_code)]

mod arguments;
mod core;
mod engine;
mod tui;
mod utils;

use arguments::Arguments;
use clap::Parser;
use engine::{Engine, TorrentSource};
use std::sync::Arc;

//use std::{env, error::Error, sync::Arc, thread, time::Instant};
//use tokio::sync::Mutex;

pub type Result<T,> = std::result::Result<T, Box<dyn std::error::Error,>,>;

fn main() -> Result<(),> {
    let args = Arguments::parse();
    args.check();

    // Creates engine
    let engine = Engine::new();
    spawn_in_engine(engine.clone(), &args,);

    tui::ui::draw_ui(engine.clone(),)?;

    Ok((),)
}

#[tokio::main(flavor = "current_thread")]
async fn spawn_in_engine(engine: Arc<Engine,>, args: &Arguments,) -> Result<(),> {
    engine
        .spawn(TorrentSource::FilePath(args.torrent_file.clone().unwrap(),),)
        .await;
    Ok((),)
}
