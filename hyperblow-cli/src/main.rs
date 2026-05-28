#![allow(non_snake_case, dead_code)]

mod arguments;
mod core;
mod engine;
mod tui;
mod utils;

use arguments::{Arguments, TorrentInput};
use clap::Parser;
use engine::{Engine, TorrentSource};
use tui::ui::TuiApplication;

//use std::{env, error::Error, sync::Arc, thread, time::Instant};
//use tokio::sync::Mutex;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args = Arguments::parse();

    // Creates engine
    let engine = Engine::new();
    if let Some(source) = args.source()? {
        StartupTorrentLoader::spawn_in_engine(engine.clone(), source)?;
    }

    TuiApplication::run_ui(engine.clone())?;

    Ok(())
}

struct StartupTorrentLoader;

impl StartupTorrentLoader {
    fn spawn_in_engine(engine: std::sync::Arc<Engine>, input: TorrentInput) -> Result<()> {
        let source = match input {
            TorrentInput::FilePath(path) => TorrentSource::FilePath(path),
            TorrentInput::MagnetUri(uri) => TorrentSource::MagnetURI(uri),
        };

        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(engine.spawn(source))?;
        Ok(())
    }
}
