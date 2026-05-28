#![allow(non_snake_case, dead_code)]

mod arguments;
mod core;
mod download_directory;
mod engine;
mod logger;
mod tui;
mod utils;

use arguments::{Arguments, TorrentInput};
use clap::Parser;
use engine::{Engine, TorrentSource};
use logger::StdoutLogger;
use tracing::{debug, info};
use tui::ui::TuiApplication;

//use std::{env, error::Error, sync::Arc, thread, time::Instant};
//use tokio::sync::Mutex;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    StdoutLogger::init_from_env();
    info!("starting hyperblow CLI");

    let args = Arguments::parse();
    debug!(
        file_source = args.torrent_file.is_some(),
        magnet_source = args.magnet_uri.is_some(),
        "parsed CLI arguments"
    );

    // Creates engine
    let engine = Engine::try_new()?;
    info!(download_directory = %engine.download_directory().display(), "engine initialized");
    if let Some(source) = args.source()? {
        StartupTorrentLoader::spawn_in_engine(engine.clone(), source)?;
    }

    info!("starting TUI");
    TuiApplication::run_ui(engine.clone())?;

    info!("hyperblow CLI exited");
    Ok(())
}

struct StartupTorrentLoader;

impl StartupTorrentLoader {
    fn spawn_in_engine(engine: std::sync::Arc<Engine>, input: TorrentInput) -> Result<()> {
        let source = match input {
            TorrentInput::FilePath(path) => TorrentSource::FilePath(path),
            TorrentInput::MagnetUri(uri) => TorrentSource::MagnetURI(uri),
        };

        info!(source = source.kind(), "spawning startup torrent");
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(engine.spawn(source))?;
        Ok(())
    }
}
