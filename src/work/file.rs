use std::sync::{Arc, Mutex};

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
                if (**x).lock().unwrap().name == *fileName {
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
    pub fn createFileTree(root_file: Arc<Mutex<File>>, files: &Vec<super::torrent_parser::File>) {
        for file in files {
            let mut working_file = root_file.clone();
            for x in 0..file.path.len() {
                let (mut idx, doesContain) =
                    (*working_file).lock().unwrap().contains(&file.path[x]);
                if !doesContain {
                    let last_path_index = file.path.len() - 1;
                    idx = Some((*working_file).lock().unwrap().add_file(File {
                        name: String::from(&file.path[x]),
                        file_type: if x == last_path_index {
                            FileType::REGULAR
                        } else {
                            FileType::DIRECTORY
                        },
                        inner_files: if x == last_path_index {
                            None
                        } else {
                            Some(vec![])
                        },
                        size: file.length,
                        should_download: true,
                    }));
                }
                if let Some(f) = &(*working_file.clone()).lock().unwrap().inner_files {
                    working_file = (*f)[idx.unwrap()].clone();
                };
            }
        }
    }

    pub fn size(&self) -> i64 {
        let mut p = 0;
        for x in self.inner_files.as_ref().unwrap() {
            println!("{:?}", x);
            println!("-----------------------------------------------------------------------------------------")
        }
        p
    }
}
