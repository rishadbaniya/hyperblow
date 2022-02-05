use crate::ui::files::FilesState;
use crate::work::file::{File, FileType};
use crate::work::torrent_parser;
use crate::work::tracker::Tracker;
use crate::Details;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Instant;

// Starting point for the parsing thread
pub fn parsing_thread_main(
    file_state: Arc<Mutex<FilesState>>,
    torrent_file_path: String,
    trackers: Arc<Mutex<Vec<RefCell<Tracker>>>>,
    details: Arc<Mutex<Details>>,
) {
    let t = Instant::now();
    let mut file_state_lock = file_state.lock().unwrap();
    let mut trackers_lock = trackers.lock().unwrap();

    // Gets the metadata from the torrent file and info_hash of the torrent
    let (file_meta, info_hash) = torrent_parser::parse_file(&torrent_file_path);
    println!(
        "Parsed torrent file : \"{}\"             [{:?}]",
        &torrent_file_path,
        Instant::now().duration_since(t)
    );

    // Sets the name of the torrent file for the UI
    let t = Instant::now();
    file_state_lock.name = file_meta.info.name.as_ref().unwrap().clone();

    // Root of the File Tree
    file_state_lock.file = Arc::new(Mutex::new(File {
        name: String::from("/"),
        file_type: FileType::DIRECTORY,
        inner_files: Some(Vec::new()),
        size: 0,
        should_download: true,
    }));

    // Creates file tree
    if let Some(x) = file_meta.info.files.as_ref() {
        // Multi file mode
        File::createFileTree(file_state_lock.file.clone(), x);
    } else {
        // Single file mode
        file_state_lock.file.lock().unwrap().inner_files = Some(vec![Arc::new(Mutex::new(File {
            name: file_meta.info.name.as_ref().unwrap().clone(),
            file_type: FileType::REGULAR,
            inner_files: None,
            size: file_meta.info.length.unwrap(),
            should_download: true,
        }))])
    }

    println!(
        "Generated File Tree                               [{:?}]",
        Instant::now().duration_since(t)
    );

    println!("Getting all the trackers socket address........");

    // Gets the socket address of all the Trackers
    let t = Instant::now();
    let announce_list: &Vec<Vec<String>> = file_meta.announce_list.as_ref().unwrap();
    *trackers_lock = Tracker::getTrackers(&file_meta.announce, announce_list);
    for tracker in &(*trackers_lock) {
        let mut tracker_borrow_mut = tracker.borrow_mut();
        if let Ok(addrs) = tracker_borrow_mut.url.socket_addrs(|| None) {
            tracker_borrow_mut.socket_adr = Some(addrs[0]);
        }
    }
    println!(
        "Got all the socket address                      [{:?}]",
        Instant::now().duration_since(t)
    );
}
