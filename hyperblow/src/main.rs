// Main thread => Draws the UI based on the working thread
// Working thread => Works on stuffs like downloading pieces and polling trackers
// Parsing thread => First thread to be run to parse the torrent file and create file tree

#![allow(non_snake_case)]

mod details;
mod ui;
mod work;

use details::Details;
use parser::metadata::FileMeta;
use std::{env, error::Error, sync::Arc, thread, time::Instant};
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

    // TODO : Use clap-rs for parsing args
    // TODO : Add a support for magnet link as an arg
    // As of right now, it just gets the path to the ".torrent" file
    let args: Vec<String> = env::args().skip(1).collect();

    // Global States that are shared across threads
    let trackers: Trackers = Vec::new(); // All the trackers in the torrent metadata
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
        crate::parse_main(
            parsing_thread_file_state,
            parsing_thread_torrent_file_path,
            parsing_thread_trackers,
            parsing_thread_details,
        )
    });

    parsing_thread.join().unwrap();

    //    //println!("This parsing staged is completed");
    //
    //    //    // Spawn worker thread
    //    //    //let working_thread_trackers = trackers.clone();
    //    //    //let working_thread_details = details.clone();
    //    //    //let working_thread_file_state = file_state.clone();
    //    //    //let working_thread = thread::spawn(move || start(working_thread_file_state, working_thread_trackers, working_thread_details));
    //    //    //working_thread.join().unwrap();
    //    //    // Draw the UI
    //    //    //ui::ui::draw_ui(file_state, details)?;
    Ok(())
}

type _FileState = Arc<Mutex<FilesState>>;
type _Trackers = Arc<Mutex<Vec<Arc<Mutex<Tracker>>>>>;
type _Details = Arc<Mutex<Details>>;

// This is the starting point of the parsing thread,
fn parse_main(file_state: _FileState, torrent_file_path: String, trackers: _Trackers, details: _Details) {
    // Sets the start of the  measuring time for parsing
    let t = Instant::now();

    // Gets the lock of all the Mutex
    //let mut lock_file_state = file_state.blocking_lock();
    //let mut lock_trackers = trackers.blocking_lock();
    let mut lock_details = details.blocking_lock();

    // Gets the metadata from the torrent file and info_hash of the torrent
    let file_meta = FileMeta::parseTorrentFile(&torrent_file_path);

    let info_hash = file_meta.get_info_hash();

    // Stores the info hash as the global state in the struct[Details]
    lock_details.info_hash = Some(info_hash);

    // Prints out the time taken to print the message
    println!(
        "Parsed torrent file : \"{}\" ----- [Time taken : {:?}]",
        &torrent_file_path,
        Instant::now().duration_since(t)
    );

    // Sets new start of the measuring time for file tree
    let t = Instant::now();
}
