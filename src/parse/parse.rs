use super::torrent_parser::parse_file;
use crate::ui::files::FilesState;
use crate::work::file::{File, FileType};
use crate::work::tracker::{self, Tracker};
use crate::Details;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::Mutex as TokioMutex;

// Starting point for the parsing thread
pub fn parsing_thread_main(
    file_state: Arc<Mutex<FilesState>>,
    torrent_file_path: String,
    trackers: Arc<TokioMutex<Vec<Arc<TokioMutex<RefCell<Tracker>>>>>>,
    details: Arc<Mutex<Details>>,
) {
    let t = Instant::now();

    // Gets the lock of all the Mutex
    let mut lock_file_state = file_state.lock().unwrap();
    let mut lock_trackers = trackers.blocking_lock();
    let mut lock_details = details.lock().unwrap();

    // Gets the metadata from the torrent file and info_hash of the torrent
    let (file_meta, info_hash) = parse_file(&torrent_file_path);
    lock_details.info_hash = Some(info_hash);

    println!(
        "Parsed torrent file : \"{}\" ----- [{:?}]",
        &torrent_file_path,
        Instant::now().duration_since(t)
    );

    let t = Instant::now();
    // Sets the name of the torrent file for the UI
    lock_details.name = Some(file_meta.info.name.as_ref().unwrap().clone());

    // Root of the File Tree
    lock_file_state.file = Arc::new(Mutex::new(File {
        name: String::from("/"),
        file_type: FileType::DIRECTORY,
        inner_files: Some(Vec::new()),
        size: 0,
        should_download: true,
    }));

    // Creates file tree
    if let Some(x) = file_meta.info.files.as_ref() {
        // Multi file mode
        File::createFileTree(lock_file_state.file.clone(), x);
    } else {
        // Single file mode
        lock_file_state.file.lock().unwrap().inner_files = Some(vec![Arc::new(Mutex::new(File {
            name: file_meta.info.name.as_ref().unwrap().clone(),
            file_type: FileType::REGULAR,
            inner_files: None,
            size: file_meta.info.length.unwrap(),
            should_download: true,
        }))])
    }
    println!("Generated File Tree ----- [{:?}]", Instant::now().duration_since(t));
    println!("Getting all the trackers socket address........");

    let t = Instant::now();
    // Gets the socket address of all the Trackers
    let announce_list: &Vec<Vec<String>> = file_meta.announce_list.as_ref().unwrap();
    //println!("{:?}", announce_list);
    //println!("{:?}", &file_meta.announce);
    *lock_trackers = Tracker::getTrackers(&file_meta.announce, announce_list);
    for tracker in &(*lock_trackers) {
        let tracker_lock = tracker.blocking_lock();
        let mut tracker_borrow_mut = tracker_lock.borrow_mut();
        if let Ok(addrs) = tracker_borrow_mut.url.socket_addrs(|| None) {
            tracker_borrow_mut.socket_adr = Some(addrs[0]);
        }
    }

    //Remove all the trackers, whose Socket Address is "None"
    *lock_trackers = (*lock_trackers)
        .iter()
        .filter(|v| v.blocking_lock().borrow().socket_adr != None)
        .map(|v| v.clone())
        .collect::<Vec<Arc<TokioMutex<RefCell<Tracker>>>>>();

    // For some unknown reason, two trackers had some Socket Address, it caused a lot of issues.
    // So, to solve this issue of having same socket address by keeping one of them only
    // We must remove that duplicate tracker with Same Socket Address

    // Store all the Sets of Index that are repeated
    let mut y: Vec<HashSet<usize>> = Vec::new();
    // 1,2,5,10
    for (i, tracker_1) in (lock_trackers).iter().enumerate() {
        let mut set: HashSet<usize> = HashSet::new();
        let socket_1 = tracker_1.blocking_lock().borrow().socket_adr.unwrap().clone();
        for (j, tracker_2) in (lock_trackers).iter().enumerate() {
            let socket_2 = tracker_2.blocking_lock().borrow().socket_adr.unwrap().clone();
            if socket_1 == socket_2 && i != j {
                set.insert(i);
                set.insert(j);
            }
        }
        if !y.contains(&set) && !set.is_empty() {
            y.push(set);
        }
    }

    println!("{:?}", y);

    let mut index_to_remove: Vec<usize> = Vec::new();
    for i in y {
        let mut z: Vec<usize> = i.into_iter().collect();
        z.pop();
        for i in z {
            index_to_remove.push(i);
        }
    }

    println!("{:?}", index_to_remove);

    let mut trackers = Vec::new();
    for (index, tracker) in (*lock_trackers).iter().enumerate() {
        if !index_to_remove.contains(&index) {
            trackers.push(tracker.clone());
        }
    }

    *lock_trackers = trackers;
    let set = println!("Got all the socket address ----- [{:?}] ", Instant::now().duration_since(t));
}
