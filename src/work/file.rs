use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum FileType {
    REGULAR,
    DIRECTORY,
}

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
}
