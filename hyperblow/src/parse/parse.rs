//use crate::ArcMutex;
//use std::collections::HashSet;
//use std::sync::Arc;
//use std::time::Instant;
//use tokio::sync::Mutex;
//
//type _FileState = Arc<Mutex<FilesState>>;
////type _Trackers = Arc<Mutex<Vec<Arc<Mutex<Tracker>>>>>;
//type _Details = Arc<Mutex<Details>>;
use hyperblow_parser::torrent_parser::FileMeta;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ArcMutex;

/// Enum that denotes the type of file
enum FileType {
    Regular,
    Directory,
}

/// DataStructure to create a file tree and perform operations on that file
struct File {
    /// Type of file, either a regular file or directory
    file_type: FileType,
    /// Inner files, if it has some, in case of (file_type as FileType::Regular), then inner_files
    /// will be empty vec
    inner_files: Vec<Rc<File>>,
    /// Size of the entire file in bytes
    size: u64,
    /// Denotes whether to download the file or not
    should_download: bool,
}

#[derive(Debug)]
pub struct TorrentFile {
    /// Path of the torrent file
    pub path: String,

    /// Info hash of the torrent
    pub info_hash: Vec<u8>,

    /// Make use of file tree for both single and multi file download,
    /// in Single File mode, there won't be any nodes whereas in Multi file mode, there
    /// can be multiple nodes
    //    fileTree: Arc<Mutex<File>>,

    /// DataStructure that holds metadata about the date encoded inside of ".torrent" file
    pub meta_info: FileMeta,
    // Global State of the torrent being downloaded
    //    state :
    //
}

impl TorrentFile {
    // TODO : Return error on error generated rather than this Option<T>
    /// It will try to parse the given the path of the torrent file and create a new data structure
    /// from the Torrent file
    pub fn new(path: &String) -> Option<TorrentFile> {
        match FileMeta::fromTorrentFile(&path) {
            Ok(meta_info) => Some(TorrentFile {
                path: path.to_string(),
                info_hash: meta_info.generateInfoHash(),
                meta_info,
            }),
            _ => None,
        }
    }

    //   fn generateFileTree(meta: FileMeta) -> Arc<Mutex<File>> {
    //       //ArcMutex!(File { file_type: , inner_files: (), size: (), should_download: () })
    //   }

    /// Starts to download the torrent, it will keep on mutating the "state" field
    /// as it progress
    async fn download(&mut self) {}
}

enum HashType {}
struct MagnetURI {
    /// URI of the Magnet link
    uri: String,

    /// Info hash of the torrent
    info_hash: Vec<u8>,

    /// Type of hash being used
    hash_type: HashType,
}

// Entry point for the parsing thread
//pub fn parsing_thread_main(file_state: _FileState, torrent_file_path: String, trackers: _Trackers, details: _Details) {
pub fn parsing_thread_main() {
    // Sets the start of the  measuring time for parsing
    //let t = Instant::now();

    // Gets the lock of all the Mutex
    //let mut lock_file_state = file_state.blocking_lock();
    //let mut lock_trackers = trackers.blocking_lock();
    //let mut lock_details = details.blocking_lock();

    // Gets the metadata from the torrent file and info_hash of the torrent
    //let file_meta = FileMeta::parseTorrentFile(&torrent_file_path);
    //let info_hash = file_meta.get_info_hash();

    //lock_details.info_hash = Some(info_hash);
    //lock_details.piece_length = file_meta.info.piece_length;

    // Sets the root of the file tree
    //    lock_file_state.file = File::createRoot();

    // Sets the root name of the torrent file for the UI
    //   lock_details.name = file_meta.info.name.clone();

    // Creates file tree
    //if let Some(x) = file_meta.info.files.as_ref() {
    // Multi file mode
    //     //   File::createFileTree(lock_file_state.file.clone(), x);
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
    //    // Sets the total size of the torrent in bytes
    //    lock_details.total_bytes = lock_file_state.file.blocking_lock().size();
    //
    //    println!("Generated File Tree ----- [Time take : {:?}]", Instant::now().duration_since(t));
    //    println!("Resolving socket address");
    //    // TODO : Show a horizontal bar for every socket address being resolved
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
}

//fn get_pieces_hash(file_meta: &FileMeta) -> Vec<[u8; 20]> {
//    let mut pieces_hash: Vec<[u8; 20]> = Vec::new();
//
//    for (i, _) in file_meta.info.pieces.iter().enumerate().step_by(20) {
//        let hash: [u8; 20] = file_meta.info.pieces[i..i + 20].try_into().unwrap();
//        pieces_hash.push(hash);
//    }
//    pieces_hash
//}
