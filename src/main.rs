// Main thread => Draws the UI based on the working thread
// Working thread => Works on stuffs like downloading pieces
// Parsing thread => First thread to be run to parse the torrent file and create file tree

#![allow(non_snake_case)]
mod parse;
mod ui;
mod work;

use std::cell::RefCell;
use std::{
    env,
    error::Error,
    sync::{Arc, Mutex},
    thread,
};
use ui::files::FilesState;
use work::{start::start as workStart, tracker::Tracker};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// Main thread to work on UI rendering
fn main() -> Result<()> {
    // Gets all the arguments
    let args: Vec<String> = env::args().skip(1).collect();

    // Global States that are shared across threads
    let file_state = Arc::new(Mutex::new(FilesState::new()));
    let trackers: Vec<RefCell<Tracker>> = Vec::new();
    let trackers = Arc::new(Mutex::new(trackers));

    // Runs the parsing thread to completion in order to
    // 1. Parse the torrent file
    // 2. Create the file tree
    // 3. Gets the socket address of all the trackers
    let parsing_thread_file_state = file_state.clone();
    let parsing_thread_torrent_file_path = args[0].clone();
    let parsing_thread_trackers = trackers.clone();
    let parsing_thread = thread::spawn(move || {
        parse::parsing_thread_main(
            parsing_thread_file_state,
            parsing_thread_torrent_file_path,
            parsing_thread_trackers,
        )
    });
    parsing_thread.join().unwrap();

    // Spawn worker thread
    let working_thread_params = (file_state.clone(), args[0].clone());
    thread::spawn(move || workStart(working_thread_params));

    // Draw the UI
    ui::ui::draw_ui(file_state)?;
    Ok(())
}
