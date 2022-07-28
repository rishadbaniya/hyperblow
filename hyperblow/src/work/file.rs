use crate::parse::torrent_parser::File as MetaFile;
use crate::ArcMutex;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum FileType {
    REGULAR,
    DIRECTORY,
}

// Struct used to create File Tree of files inside of a torrent
#[derive(Debug)]
pub struct File {
    // Name of the file
    pub name: String,
    // Type of File
    pub file_type: FileType,
    // Nested File Nodes if its a directory
    pub inner_files: Option<Vec<Arc<Mutex<File>>>>,
    // Size of file
    pub size: i64,
    // Whether to download the file or not
    // If it's a folder, then it automatically sets all the children nodes to false
    pub should_download: bool,
}

impl File {
    /// We'll have one root file (type : Directory), which will have root size of 0, it's only
    /// purpose is to nest the file/s to be downloaded, nothing else, it's basically initial wrapper
    /// around the files to be downloaded
    ///
    /// Sets the root of the File Tree
    ///
    /// "/" means its the root file and anything to be downloaded will be under it
    ///
    /// It's size is set to 0, coz it's just a file made to nest other files, it doesn't exist and
    /// doesn't have any intrinsic size
    pub fn createRoot() -> Arc<Mutex<Self>> {
        ArcMutex! {File {
         name: String::from("/"),
         file_type: FileType::DIRECTORY,
         inner_files: Some(Vec::new()),
         size: 0,
         should_download: true,
        }}
    }
    // Checks if the File Node contains file with given name, upto just 1 level depth
    //
    //    X     -> Root File Node on which the method is called
    //  /   \
    // Y     Z  -> Tree Level on which this method checks
    //
    // and returns the index of the file in that node and whether it exists or not
    pub fn contains(&self, fileName: &String) -> (Option<usize>, bool) {
        let mut index = None;
        let mut doesExist = false;
        if let Some(files) = &self.inner_files {
            for (i, x) in files.iter().enumerate() {
                if (**x).blocking_lock().name == *fileName {
                    index = Some(i);
                    doesExist = true;
                }
            }
        }
        (index, doesExist)
    }

    // Push the new Node inside of the Node that calls this method and returns the index of the
    // pushed node
    pub fn add_file(&mut self, file: File) -> usize {
        let mut index = 0;
        if let Some(i) = &mut self.inner_files {
            i.push(Arc::new(Mutex::new(file)));
            index = i.len() - 1
        }
        index
    }

    pub fn changeShouldDownload(&mut self) {
        let currentDownloadState = self.should_download;
        self.should_download = !currentDownloadState;
    }

    // Creates a File Tree inside of given File Node
    // Used to create a file tree inside of a root File instance
    pub fn createFileTree(root_file: Arc<Mutex<File>>, files: &Vec<MetaFile>) {
        for file in files {
            let mut working_file = root_file.clone();
            for x in 0..file.path.len() {
                let (mut idx, doesContain) = (*working_file).blocking_lock().contains(&file.path[x]);
                if !doesContain {
                    let last_path_index = file.path.len() - 1;
                    idx = Some((*working_file).blocking_lock().add_file(File {
                        name: String::from(&file.path[x]),
                        file_type: if x == last_path_index { FileType::REGULAR } else { FileType::DIRECTORY },
                        inner_files: if x == last_path_index { None } else { Some(vec![]) },
                        size: file.length,
                        should_download: true,
                    }));
                }
                if let Some(f) = &(*working_file.clone()).blocking_lock().inner_files {
                    working_file = (*f)[idx.unwrap()].clone();
                };
            }
        }
    }

    /// Gets the total size of the File, if it's a "direcotry file" then it gives the total size of all the Files within that File tree
    /// Note : Its recursive
    pub fn size(&self) -> i64 {
        let mut size = 0;
        if let Some(files) = &self.inner_files {
            size += self.size;
            for file in files {
                size += file.blocking_lock().size();
            }
        } else {
            size += self.size;
        }
        size
    }
}
