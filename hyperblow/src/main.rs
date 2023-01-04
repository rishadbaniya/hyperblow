// Main thread => Draws the UI based on the working thread
// Working thread => Works on stuffs like downloading pieces and polling trackers
// Parsing thread => First thread to be run to parse the torrent file and create file tree

#![allow(non_snake_case)]

mod parse;

use clap::Parser;
use parse::parse::TorrentFile;

//use std::{env, error::Error, sync::Arc, thread, time::Instant};
//use tokio::sync::Mutex;

#[derive(Debug, Parser, Default)]
#[clap(author = "Rishad Baniya", version)]
struct Arguments {
    #[arg(short('f'))]
    /// Path to the torrent file you wish to download
    torrent_file: Option<String>,

    /// URI of the torrent file you wish to download
    #[arg(short('u'))]
    magnet_uri: Option<String>,
}

//mod details;
//mod ui;
//mod work;
//
//use details::Details;
//use parser::metadata::FileMeta;
//use std::collections::HashSet;

//use ui::files::FilesState;
//use work::file::{File, FileType};
//use work::tracker::Tracker;
//

#[macro_export]
macro_rules! ArcMutex {
    ($e : expr) => {
        Arc::new(Mutex::new($e))
    };
}
//
////type Trackers = Vec<Arc<Mutex<Tracker>>>;
//type Result<T> = std::result::Result<T, Box<dyn Error>>;
////
////// Main thread to work on UI rendering

fn main() {
    let args = Arguments::parse();
    if let Some(ref path) = args.torrent_file {
        let x = TorrentFile::new(path);
    }

    // As of right now i'll consider that we're using this application to handle
    // just one torrent download and not mutiple

    // Global States that are shared across threads

    //let trackers: Trackers = Vec::new(); // All the trackers in the torrent metadata
    //let details = ArcMutex!(Details::default());
    //let file_state = ArcMutex!(FilesState::new());
    //let trackers = ArcMutex!(trackers);
    //
    // Spawn and run the parsing thread to "completion", blocking the "main thread" in order to
    // 1. Parse the torrent file
    // 2. Create the file tree
    // 3. Get the socket address of all the trackers
    // 4. Remove the trackers who have same socket address (Motivation : I found it that two
    //    UDP trackers with different domain names had resolved to same socket adress and this
    //    causes multiple Connect Request to be, creating issues)
    //
    //    let parsing_thread_details = details.clone();
    //    let parsing_thread_file_state = file_state.clone();
    //    let parsing_thread_torrent_file_path = args[0].clone();
    //    let parsing_thread_trackers = trackers.clone();
    //    let parsing_thread = thread::spawn(move || {
    //        crate::parse_main(
    //            parsing_thread_file_state,
    //            parsing_thread_torrent_file_path,
    //            parsing_thread_trackers,
    //            parsing_thread_details,
    //        )
    //    });
    //
    //    println!("This parsing staged is completed");
    //
    //    parsing_thread.join().unwrap();
    //
    //    // Spawn worker thread
    //    let working_thread_trackers = trackers.clone();
    //    let working_thread_details = details.clone();
    //    let working_thread_file_state = file_state.clone();
    //    //let working_thread = thread::spawn(move || start(working_thread_file_state, working_thread_trackers, working_thread_details));
    //    //working_thread.join().unwrap();
    //
    //    //Draw the UI
    //    //ui::ui::draw_ui(file_state, details)?;
    //    Ok(())
    //}
    //
    //type _FileState = Arc<Mutex<FilesState>>;
    //type _Trackers = Arc<Mutex<Vec<Arc<Mutex<Tracker>>>>>;
    //type _Details = Arc<Mutex<Details>>;
    //
    //// This is the starting point of the parsing thread,
    //fn parse_main(file_state: _FileState, torrent_file_path: String, trackers: _Trackers, details: _Details) {
    //    let t = Instant::now(); // Sets the start of the measuring time for parsing
    //
    //    // Gets the lock of all the Mutex
    //    let mut lock_file_state = file_state.blocking_lock();
    //    let mut lock_trackers = trackers.blocking_lock();
    //    let mut lock_details = details.blocking_lock();
    //
    //    let file_meta = FileMeta::parseTorrentFile(&torrent_file_path); // Gets the metadata from the torrent file
    //    let info_hash = file_meta.get_info_hash();
    //    lock_details.info_hash = Some(info_hash); // Stores the info hash as global state in teh struct[Details]
    //
    //    // Prints out the time taken to print the message
    //    println!(
    //        "Parsed torrent file : \"{}\" ----- [Time taken : {:?}]",
    //        &torrent_file_path,
    //        Instant::now().duration_since(t)
    //    );
    //
    //    let t = Instant::now(); // Sets the new start of the measuring tiem for file tree
    //
    //    lock_file_state.file = File::createRoot(); // Sets the root of the file tree
    //    lock_details.name = file_meta.info.name.clone(); // Sets the root name of the torrent file for the UI
    //
    //    // Creates file tree
    //    if let Some(x) = file_meta.info.files.as_ref() {
    //        // Multi file mode
    //        File::createFileTree(lock_file_state.file.clone(), x);
    //    } else {
    //        // Single file mode
    //        lock_file_state.file.blocking_lock().inner_files = Some(vec![ArcMutex! { File {
    //            name: file_meta.info.name.as_ref().unwrap().clone(),
    //            file_type: FileType::REGULAR,
    //            inner_files: None,
    //            size: file_meta.info.length.unwrap(),
    //            should_download: true,
    //        }}])
    //    }
    //
    //    lock_details.total_bytes = lock_file_state.file.blocking_lock().size(); // Sets the total size of the torrent in bytes
    //
    //    println!("Generated File Tree ----- [Time take : {:?}]", Instant::now().duration_since(t));
    //    println!("Resolving socket address");
    //
    //    // Try to Resolve the socket address of all the Trackers
    //    let announce_list: &Vec<Vec<String>> = file_meta.announce_list.as_ref().unwrap();
    //
    //    *lock_trackers = Tracker::getTrackers(&file_meta.announce, announce_list);
    //    for tracker in &(*lock_trackers) {
    //        let mut tracker_lock = tracker.blocking_lock();
    //        if let Ok(addrs) = tracker_lock.url.socket_addrs(|| None) {
    //            tracker_lock.socket_adr = Some(addrs[0]);
    //        }
    //    }
    //
    //    //Remove all the trackers, whose Socket Address is "None"
    //    *lock_trackers = (*lock_trackers)
    //        .iter()
    //        .filter(|v| v.blocking_lock().socket_adr != None)
    //        .map(|v| v.clone())
    //        .collect::<Vec<Arc<Mutex<Tracker>>>>();
    //
    //    // For some unknown reason, two trackers had some Socket Address, it caused a lot of issues.
    //    // So, to solve this issue of having same socket address by keeping one of them only
    //    // We must remove that duplicate tracker with Same Socket Address
    //
    //    // Store all the Sets of Index that are repeated
    //    let mut y: Vec<HashSet<usize>> = Vec::new();
    //    for (i, tracker_1) in (lock_trackers).iter().enumerate() {
    //        let mut set: HashSet<usize> = HashSet::new();
    //        let socket_1 = tracker_1.blocking_lock().socket_adr.unwrap().clone();
    //        for (j, tracker_2) in (lock_trackers).iter().enumerate() {
    //            let socket_2 = tracker_2.blocking_lock().socket_adr.unwrap().clone();
    //            if socket_1 == socket_2 && i != j {
    //                set.insert(i);
    //                set.insert(j);
    //            }
    //        }
    //        if !y.contains(&set) && !set.is_empty() {
    //            y.push(set);
    //        }
    //    }
    //
    //    let mut index_to_remove: Vec<usize> = Vec::new();
    //    for i in y {
    //        let mut z: Vec<usize> = i.into_iter().collect();
    //        z.pop();
    //        for i in z {
    //            index_to_remove.push(i);
    //        }
    //    }
    //
    //    let mut trackers = Vec::new();
    //    for (index, tracker) in (*lock_trackers).iter().enumerate() {
    //        if !index_to_remove.contains(&index) {
    //            trackers.push(tracker.clone());
    //        }
    //    }
    //    *lock_trackers = trackers;
    //
    //    lock_details.pieces_hash.append(&mut get_pieces_hash(&file_meta));
    //
    //    // Total of of hash is same as total of pieces
    //    lock_details.total_pieces = lock_details.pieces_hash.len() as u32;
    //}
    //
    //fn get_pieces_hash(file_meta: &FileMeta) -> Vec<[u8; 20]> {
    //    let mut pieces_hash: Vec<[u8; 20]> = Vec::new();
    //
    //    for (i, _) in file_meta.info.pieces.iter().enumerate().step_by(20) {
    //        let hash: [u8; 20] = file_meta.info.pieces[i..i + 20].try_into().unwrap();
    //        pieces_hash.push(hash);
    //    }
    //    pieces_hash
}
