// NOTE: When i say files, i mean it in Unix term. Folders are files as well, their type is Directory

use super::torrent_parser;
use crate::ui::files::FilesState;
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
    fn contains(&self, fileName: &String) -> (Option<usize>, bool) {
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
    fn add_file(&mut self, file: File) -> usize {
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

// Starting Point for the working thread
pub fn start(fileState: Arc<Mutex<FilesState>>, torrent_file_path: &String) {
    // Get the argument at index 1 from the CLI command "rtourent xyz.torrent"
    // So that we can get the name of the file i.e xyz.torrentj
    let (torrentParsed, info_hashBytes) = torrent_parser::parse_file(&torrent_file_path);

    let x = std::time::Instant::now();
    // Root file to store all the files
    fileState.lock().unwrap().file = Arc::new(Mutex::new(File {
        name: String::from("/"),
        file_type: FileType::DIRECTORY,
        inner_files: Some(Vec::new()),
        size: 0,
        should_download: true,
    }));

    if let Some(files) = &torrentParsed.info.files {
        for file in files {
            let mut afile = fileState.lock().unwrap().file.clone();
            for x in 0..file.path.len() {
                let (mut idx, doesContain) = (*afile).lock().unwrap().contains(&file.path[x]);
                if !doesContain {
                    let last_path_index = file.path.len() - 1;
                    idx = Some((*afile).lock().unwrap().add_file(File {
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
                if let Some(f) = &(*afile.clone()).lock().unwrap().inner_files {
                    afile = (*f)[idx.unwrap()].clone();
                };
            }
        }
    }

    {
        if let Some(x) = &(*fileState.lock().unwrap().file.clone())
            .lock()
            .unwrap()
            .inner_files
        {
            for xx in x {
                println!("{:?}", xx);
            }
        }
    }

    println!(
        "{}",
        std::time::Instant::now().duration_since(x).as_micros()
    )

    //   let percentEncodedInfoHash = percent_encoder::encode(info_hashBytes);
    //  println!("{:?}", percentEncodedInfoHash);
    //
    //    let client = Client::new();
    //    let uri = format!(
    //        "{}?info_hash={}&peer_id=RISHADBANIYA_1234567&port=6881",
    //        &torrentParsed.announce, &percentEncodedInfoHash
    //    )
    //    .parse()?;
    //    println!("{}", uri);
    //
    //    let resp = client.get(uri).await?;
    //    let body: Vec<u8> = (hyper::body::to_bytes(resp.into_body()).await?)
    //        .into_iter()
    //        .collect();
    //
    //    let tracker_response: TrackerResponse = serde_bencode::de::from_bytes(&body)?;
    //    println!("{}", String::from_utf8_lossy(&body));
    //    println!("{:?}", tracker_response);
    //
    //    let mut allTrackers: Vec<String> = vec![torrentParsed.announce.clone()];
    //
    //    if let Some(announce_list) = torrentParsed.announce_list {
    //        for tracker in announce_list {
    //            allTrackers.push(tracker[0].clone());
    //        }
    //    }
    //
    //    println!("All trackers are {:?}", allTrackers);
    //
    //    Ok(())
}

//#[derive(Debug, Deserialize)]
//struct TrackerResponse {
//    #[serde(rename = "failure reason")]
//    failure_reason: Option<String>,
//    #[serde(rename = "warning message")]
//    warning_message: Option<String>,
//    interval: Option<i64>,
//    #[serde(rename = "min interval")]
//   min_interval: Option<i64>,
//    #[serde(rename = "tracker id")]
//    tracker_id: Option<String>,
//    complete: Option<i64>,
//    incomplete: Option<i64>,
//    peers: Vec<Peers>,
//}
//
//#[derive(Debug, Deserialize)]
//struct Peers {
//    #[serde(rename = "peer id")]
//    peer_id: String,
//    ip: String,
//    port: String,
//}
