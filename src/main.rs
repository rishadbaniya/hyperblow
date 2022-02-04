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

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// Main thread to work on UI rendering
fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    let app_state = Arc::new(Mutex::new(FilesState::new()));

    // Spawn worker thread
    let appStateWorkingThread = app_state.clone();
    let handle = thread::spawn(move || workStart(appStateWorkingThread, &args[0]));
    handle.join().unwrap();

    // Draw the UI
    ui::ui::draw_ui(app_state)?;
    Ok(())
}
