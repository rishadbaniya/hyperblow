use crate::torrent_parser::FileMeta;

pub fn spit_details(fileMeta: &FileMeta) {
    println!("------------------------------- Torrent Details -------------------------------");

    if let Some(name) = &fileMeta.info.name {
        println!("Name -> {}", name);
    }

    if let Some(allFiles) = &fileMeta.info.files {
        println!("Total Files -> {}", allFiles.len());
    }
    println!("Total Folders -> ")
}
