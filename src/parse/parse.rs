use crate::ui::files::FilesState;
use crate::work::file::{File, FileType};
use crate::work::torrent_parser;
use std::sync::{Arc, Mutex};

pub fn parsing_thread_main(file_state: Arc<Mutex<FilesState>>, torrent_file_path: String) {
    let mut file_state_lock = file_state.lock().unwrap();

    // Gets the metadata from the torrent file and info_hash of the torretnt
    let (file_meta, info_hash) = torrent_parser::parse_file(&torrent_file_path);
    println!("Parsed torrent file : \"{}\"", &torrent_file_path);

    // Sets the name of the torrent file for the UI
    file_state_lock.name = file_meta.info.name.as_ref().unwrap().clone();

    // Root of the File Tree
    file_state_lock.file = Arc::new(Mutex::new(File {
        name: String::from("/"),
        file_type: FileType::DIRECTORY,
        inner_files: Some(Vec::new()),
        size: 0,
        should_download: true,
    }));

    if let Some(x) = file_meta.info.files.as_ref() {
        // Creates file tree for multi file mode
        File::createFileTree(file_state_lock.file.clone(), x);
    } else {
        // Creates file tree for single file mode
        file_state_lock.file.lock().unwrap().inner_files = Some(vec![Arc::new(Mutex::new(File {
            name: file_meta.info.name.as_ref().unwrap().clone(),
            file_type: FileType::REGULAR,
            inner_files: None,
            size: file_meta.info.length.unwrap(),
            should_download: true,
        }))])
    }

    println!("Generated File Tree");
}
