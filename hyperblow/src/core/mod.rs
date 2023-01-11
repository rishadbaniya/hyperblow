mod state;
mod torrentFile;
mod tracker;

use hyperblow_parser::torrent_parser::FileMeta;
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use std::sync::Arc;
use tokio::sync::Mutex;
pub use torrentFile::TorrentFile;

#[macro_export]
macro_rules! ArcMutex {
    ($e : expr) => {
        Arc::new(Mutex::new($e))
    };
}

/// Enum that denotes the type of file
#[derive(Debug, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
}

/// DataStructure to create a file tree and perform operations on that file
#[derive(Debug)]
pub struct File {
    /// Name of the file
    pub name: String,

    /// Type of file, either a regular file or directory
    pub file_type: FileType,

    /// Inner files, if it has some, in case of (file_type as FileType::Regular), then there will
    /// be Some(Vec<Rc<File>>) else there will be none
    pub inner_files: Option<Vec<Arc<Mutex<File>>>>,

    /// Size of the entire file in bytes
    /// Directory will be given size of None, whereas the actual files will be given size of
    /// Some(i64) where the size is in bytes
    pub size: Option<i64>,

    /// Denotes whether to download the file or not
    pub should_download: bool,

    /// Denotes the progress in percentage
    pub progressPerc: f32,

    /// Will be turned to downloaded when progressPerc reaches 100
    pub isDownloaded: bool,
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

// TODO: Make use of AsRef
/// Encode the given byte vector of info_hash into a String of
/// Percent Encoded info_hash
pub fn percEncode(byteVector: Vec<u8>) -> String {
    percent_encode(&byteVector, NON_ALPHANUMERIC).to_string()
}
