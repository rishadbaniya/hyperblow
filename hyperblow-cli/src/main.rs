#![allow(non_snake_case, dead_code)]

mod arguments;
mod core;
mod engine;
mod tui;
mod utils;

use arguments::{Arguments, TorrentInput};
use clap::Parser;
use engine::{Engine, TorrentSource};
use std::sync::Arc;

//use std::{env, error::Error, sync::Arc, thread, time::Instant};
//use tokio::sync::Mutex;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args = Arguments::parse();

    // Creates engine
    let engine = Engine::new();
    if let Some(source) = args.source()? {
        spawn_in_engine(engine.clone(), source)?;
    }

    tui::ui::draw_ui(engine.clone())?;

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn spawn_in_engine(engine: Arc<Engine>, input: TorrentInput) -> Result<()> {
    let source = match input {
        TorrentInput::FilePath(path) => TorrentSource::FilePath(path),
        TorrentInput::MagnetUri(uri) => TorrentSource::MagnetURI(uri),
    };
    engine.spawn(source).await?;
    Ok(())
}
