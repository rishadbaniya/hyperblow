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
use rand::seq::SliceRandom;
use rand::thread_rng;
use reqwest::Url;
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ArcMutex;

//use crate::ArcMutex;

/// Enum that denotes the type of file
#[derive(Debug, PartialEq, Eq)]
enum FileType {
    Regular,
    Directory,
}

/// DataStructure to create a file tree and perform operations on that file
#[derive(Debug)]
pub struct File {
    /// Name of the file
    name: String,

    /// Type of file, either a regular file or directory
    file_type: FileType,

    /// Inner files, if it has some, in case of (file_type as FileType::Regular), then there will
    /// be Some(Vec<Rc<File>>) else there will be none
    inner_files: Option<Vec<Arc<Mutex<File>>>>,

    /// Size of the entire file in bytes
    /// Directory will be given size of None, whereas the actual files will be given size of
    /// Some(i64) where the size is in bytes
    size: Option<i64>,

    /// Denotes whether to download the file or not
    should_download: bool,

    /// Denotes the progress in percentage
    progressPerc: f32,

    /// Will be turned to downloaded when progressPerc reaches 100
    isDownloaded: bool,
}

impl File {
    // TODO : Generate file tree based on the data inside of ".torrent" file and the resumable data
    // as well
    // Generates a file tree based on the data inside of ".torrent" file
    // meta => It's the File Meta that has all the informations about the torrent file
    // directory => The download directory of the data i.e the absolute path of the directory
    // where we want the contents to go to
    //
    pub fn new(meta: &FileMeta, directory: &String) -> Result<Arc<Mutex<File>>, Box<dyn std::error::Error>> {
        // Create file tree in single file mode
        let mut rootFile = File {
            name: directory.to_owned(),
            file_type: FileType::Directory,
            inner_files: Some(Vec::new()),
            size: None,
            should_download: true,
            progressPerc: 0_f32,
            isDownloaded: false,
        };

        println!("{:?}", meta.info.name);

        if let Some(ref files) = meta.info.files {
            let rootFile = ArcMutex!(rootFile);
            println!("Entered multiple file mode");
            // Multiple file mode
            // Go through all the files inside of meta.info.files given by the ".torrent" file
            let mut currentFile = rootFile.clone();
            for f in files {
                // The eventual path of the file, will also include the directory
                let ref path_s = f.path;
                for (ind, path) in path_s.into_iter().enumerate() {
                    let containsAtDepthOne = { currentFile.blocking_lock().containsAtDepthOne(path) };
                    match containsAtDepthOne {
                        Some(i) => {
                            let curFile = { currentFile.blocking_lock().inner_files.as_ref().unwrap().get(i).unwrap().clone() };
                            currentFile = curFile;
                        }
                        None => {
                            let curFile = {
                                let mut currentFileLock = currentFile.blocking_lock();
                                let file_type = if (path_s.len() - 1) == ind || path_s.len() == 1 {
                                    FileType::Regular
                                } else {
                                    FileType::Directory
                                };

                                let size = if file_type == FileType::Regular { Some(f.length) } else { None };
                                currentFileLock.constructDirectoryOrFile(path, file_type, size);
                                let inner_files = currentFileLock.inner_files.as_ref().unwrap();
                                inner_files[inner_files.len() - 1].clone()
                            };
                            currentFile = curFile;
                        }
                    }
                }
                currentFile = rootFile.clone();
            }
            Ok(rootFile)
        } else {
            // Single File Mode
            rootFile.file_type = FileType::Regular;
            rootFile.inner_files = None;

            Ok(ArcMutex!(rootFile))
        }
    }

    fn constructDirectoryOrFile(&mut self, fileOrFolderName: &String, file_type: FileType, size: Option<i64>) {
        if let Some(ref mut inner_files) = self.inner_files {
            inner_files.push(ArcMutex!(File {
                name: fileOrFolderName.to_owned(),
                progressPerc: 0_f32,
                should_download: true,
                size: size, // TODO : Use actual size
                isDownloaded: false,
                inner_files: if file_type == FileType::Regular { None } else { Some(Vec::new()) },
                file_type,
            }));
        }
    }

    fn containsAtDepthOne(&self, fileOrFolderName: &String) -> Option<usize> {
        if let Some(ref inner_files) = self.inner_files {
            for (i, file) in inner_files.into_iter().enumerate() {
                let name = { file.blocking_lock().name.clone() };
                if name == *fileOrFolderName {
                    return Some(i);
                }
            }
        } else {
            if self.name == *fileOrFolderName {
                return Some(0);
            }
        }
        return None;
    }
}

// There is a chance that this DataStructure is going to be accessed many times per seconds, so its
// better that its field will be access and mutated rather than locking the entire data structure
// in a Mutex
//
// It contains the filetree that will be constructed on resume and even starting of the download
// phase, and will be constantly updated on each file information
//
// TODO : Make sure the learning i did was correct
use std::net::SocketAddr;
#[derive(Debug)]
pub struct Tracker {
    address: Url,
    /// A single domain can have multiple socket address, i.e it can resolve to multiple ip address
    socketAddrs: Option<Vec<SocketAddr>>,
}

impl Tracker {
    /// Tries to create a Tracker instance by parsing the given url
    fn new(address: &String) -> Result<Tracker, Box<dyn std::error::Error>> {
        let address = Url::parse(address)?;
        Ok(Tracker { address, socketAddrs: None })
    }

    fn resolveSocketAddr(&mut self) -> bool {
        if let Ok(addrs) = self.address.socket_addrs(|| None) {
            self.socketAddrs = Some(addrs);
            true
        } else {
            false
        }
    }
}

// TODO : Figure out a way to find which piece index and its data falls under a certain file or
// folder we want to download and how should one approach to download that shit
// TODO : Create a file tree generated from reading the torrent file
// TODO : Add Global State, by global state i mean add those values that change during the runtime
// and is crucial to show to the user as well, such as downloaded pieces, their index, trackers and
// their iformations and all other data related to it
// TODO : Find the folder to save the data
// TODO : Create the DataStructure in such a way that it could resume the download later on as well
#[derive(Debug)]
pub struct TorrentFile {
    /// Path of the torrent file
    pub path: String,

    /// Info hash of the torrent
    pub info_hash: Vec<u8>,

    /// DataStructure that holds metadata about the date encoded inside of ".torrent" file
    pub meta_info: FileMeta,

    /// Stores the hash of each piece by its exact index extracted out of bencode encoded ".torrent" file
    pub pieces_hash: Vec<[u8; 20]>,

    /// Stores the total no of pieces
    pub piecesCount: usize,

    pub totalSize: usize,

    pub fileTree: Arc<Mutex<File>>,

    pub trackers: Arc<Mutex<Vec<Vec<Tracker>>>>,
}

/// TODO: Implement DHT(Distributed Hash Table) as well
impl TorrentFile {
    // TODO : Return error on error generated rather than this Option<T>
    /// It will try to parse the given the path of the torrent file and create a new data structure
    /// from the Torrent file
    pub fn new(path: &String) -> Option<TorrentFile> {
        match FileMeta::fromTorrentFile(&path) {
            Ok(meta_info) => {
                let info_hash = meta_info.generateInfoHash();
                let pieces_hash = meta_info.getPiecesHash();

                Some(TorrentFile {
                    path: path.to_string(),
                    info_hash,
                    piecesCount: pieces_hash.len(),
                    pieces_hash,
                    fileTree: TorrentFile::generateFileTree(&meta_info),
                    trackers: ArcMutex!(vec![]),
                    meta_info,
                    totalSize: 0, // TODO : Replace it with actual total size of the torrent
                })
            }
            _ => None,
        }
    }

    /// Creates objects of [Tracker] by extracting out all the Trackers from "announce" and "announce-list" field
    /// and then resolves their address through DNS lookup
    pub fn resolveTrackers(&self) {
        // According to BEP12, if announce_list field is present then the client will have to
        // ignore the announce field as the URL in the announce field is already present in the
        // announce_list
        //
        //
        // Step -1 :
        //
        // Create a Tracker instance, parse the given string and then resolve the socket
        // address for both cases, when there is only announce field and when there is
        // announce_list field as well
        if let Some(ref d) = self.meta_info.announce_list {
            for i in d {
                let x: Vec<Tracker> = {
                    let mut trackers = vec![];
                    for addrs in i {
                        // This tries to parse the given URL and if it parses successfully then
                        // signals to resolve the socket address
                        if let Ok(mut tracker) = Tracker::new(addrs) {
                            tracker.resolveSocketAddr();
                            trackers.push(tracker);
                        }
                    }
                    trackers
                };
                self.trackers.blocking_lock().push(x);
            }
        } else {
            if let Ok(mut tracker) = Tracker::new(&self.meta_info.announce) {
                tracker.resolveSocketAddr();
                self.trackers.blocking_lock().push(vec![tracker]);
            }
        }
    }

    pub fn generateFileTree(meta: &FileMeta) -> Arc<Mutex<File>> {
        File::new(meta, &"root".to_owned()).unwrap()
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
