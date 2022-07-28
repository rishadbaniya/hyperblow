// Main thread => Draws the UI based on the working thread
// Working thread => Works on stuffs like downloading pieces and polling trackers
// Parsing thread => First thread to be run to parse the torrent file and create file tree
#![allow(non_snake_case)]

mod details;
mod parse;
mod ui;
mod work;

use details::Details;
use std::{env, error::Error, sync::Arc, thread};
use tokio::sync::Mutex;
use ui::files::FilesState;
use work::tracker::Tracker;

#[macro_export]
macro_rules! ArcMutex {
    ($e : expr) => {
        Arc::new(Mutex::new($e))
    };
}

type Trackers = Vec<Arc<Mutex<Tracker>>>;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

// Main thread to work on UI rendering
fn main() -> Result<()> {
    // Gets all the arguments
    //
    // TODO : Use clap-rs for parsing args
    // TODO : Add a support for magnet link as an arg
    // As of right now, it just gets the path to the ".torrent" file
    let args: Vec<String> = env::args().skip(1).collect();

    // Global States that are shared across threads
    let trackers: Trackers = Vec::new();
    let details = ArcMutex!(Details::default());
    let file_state = ArcMutex!(FilesState::new());
    let trackers = ArcMutex!(trackers);

    // Spawn and run the parsing thread to "completion", blocking the "main thread" in order to
    // 1. Parse the torrent file
    // 2. Create the file tree
    // 3. Get the socket address of all the trackers
    // 4. Remove the trackers who have same socket address (Motivation : I found it that two
    //    UDP trackers with different domain names had resolved to same socket adress and this
    //    causes multiple Connect Request to be, creating issues)
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
    println!("This parsing staged is completed");

    // Spawn worker thread
    //let working_thread_trackers = trackers.clone();
    //let working_thread_details = details.clone();
    //let working_thread_file_state = file_state.clone();
    //let working_thread = thread::spawn(move || start(working_thread_file_state, working_thread_trackers, working_thread_details));
    //working_thread.join().unwrap();
    // Draw the UI
    //ui::ui::draw_ui(file_state, details)?;
    Ok(())
}
