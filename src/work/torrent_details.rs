use crate::torrent_parser::FileMeta;

pub fn spit_details(fileMeta: &FileMeta) {
    println!("------------------------------- Torrent Details -------------------------------");

    if let Some(x) = &fileMeta.info.files {
        for y in x {
            println!("{:?}", y.path);
        }
    }

    if let Some(name) = &fileMeta.info.name {
        println!("Name -> {}", name);
    }

    if let Some(allFiles) = &fileMeta.info.files {
        // In FileMeta.info.files, the vector contains list of all vectors which contains
        // information of directory and at last file name
        // Eg. [["a","b","c.txt"],[["a"],["d.txt"]] => It means a/b/c.txt and a/d.txt
        // So getting every vector within this vector has a file name at its last index.
        // Getting the length of this parent vector will give us all total no. of files
        let totalFiles = allFiles.len();

        // Declaring and initializing  an empty vector to store all unique folders name
        let mut allFolders: Vec<String> = Vec::new();

        //Get all the unique folders from the allFiles Vector
        for x in allFiles {
            if x.path.len() > 1 {
                for z in 0..(x.path.len() - 1) {
                    if !allFolders.contains(&x.path[z]) {
                        allFolders.push(x.path[z].clone());
                    }
                }
            }
        }
        println!("Total Files -> {}", totalFiles);
        println!("Total Folders -> {}", allFolders.len());
    } else {
        // If the above one doesn't work, it means there is just one file to download
        // See : https://wiki.theory.org/index.php/BitTorrentSpecification | Info in Multiple File Mode
        // The code above is for multiple files mode, if it doesnt work then it means
        // there is just one file
        println!("Total Files -> 1");
        println!("Total Folders -> 0");
    }
}
