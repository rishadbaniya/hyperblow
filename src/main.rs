#![allow(non_snake_case)]
mod ui;
mod work;

use std::{
    env,
    error::Error,
    sync::{Arc, Mutex},
    thread,
};
use ui::files::FilesState;
use work::start::start as workStart;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

// Main thread to work on UI rendering
fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    // Global State Of the App that is Shared Across Threads
    let appState = Arc::new(Mutex::new(FilesState::new()));

    // Spawn worker thread
    let appStateWorkingThread = appState.clone();
    let handle = thread::spawn(move || workStart(appStateWorkingThread, &args[0]));
    handle.join().unwrap();

    // Draw the UI
    ui::ui::draw_ui(appState)?;
    Ok(())
}
