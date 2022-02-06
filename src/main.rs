#![allow(non_snake_case)]

// Main thread => Draws the UI based on the working thread
// Working thread => Works on stuffs like downloading pieces and polling trackers
// Parsing thread => First thread to be run to parse the torrent file and create file tree

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

// Struct that holds the state for Details Section of the UI
#[derive(Debug, Clone)]
pub struct Details {
    name: Option<String>,
    info_hash: Option<Vec<u8>>,
}

impl Default for Details {
    fn default() -> Self {
        let name = None;
        let info_hash = None;
        Self { name, info_hash }
    }
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// Main thread to work on UI rendering
fn main() -> Result<()> {
    // Gets all the arguments
    let args: Vec<String> = env::args().skip(1).collect();

    // Global States that are shared across threads
    let trackers: Vec<Arc<Mutex<RefCell<Tracker>>>> = Vec::new();
    let details = Arc::new(Mutex::new(Details::default()));
    let file_state = Arc::new(Mutex::new(FilesState::new()));
    let trackers = Arc::new(Mutex::new(trackers));

    // Spawn and run the parsing thread to "completion", blocking the "main thread" in order to
    // 1. Parse the torrent file
    // 2. Create the file tree
    // 3. Get the socket address of all the trackers
    let parsing_thread_details = details.clone();
    let parsing_thread_file_state = file_state.clone();
    let parsing_thread_torrent_file_path = args[0].clone();
    let parsing_thread_trackers = trackers.clone();
    let parsing_thread = thread::spawn(move || {
        parse::parsing_thread_main(
            parsing_thread_file_state,
            parsing_thread_torrent_file_path,
            parsing_thread_trackers,
            parsing_thread_details,
        )
    });
    parsing_thread.join().unwrap();

    // Spawn worker thread
    let working_thread_trackers = trackers.clone();
    let working_thread_details = details.clone();
    let working_thread_params = (file_state.clone(), args[0].clone());
    let working_thread = thread::spawn(move || {
        workStart(
            working_thread_params,
            working_thread_trackers,
            working_thread_details,
        )
    });

    working_thread.join();

    // Draw the UI
    ui::ui::draw_ui(file_state)?;
    Ok(())
}
