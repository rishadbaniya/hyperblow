// NOTE: When i say files, i mean it in Unix term. Folders are files as well, their type is Directory

use super::tracker::{annnounce_request, connect_request};
use super::tracker::{ConnectResponse, Tracker};
use super::{file, torrent_parser};
use crate::{
    ui::files::FilesState,
    work::{
        file::File,
        tracker::{AnnounceRequest, AnnounceResponse, ConnectRequest, TrackerProtocol},
    },
};
use std::sync::{Arc, Mutex};
use std::{net::SocketAddr, time::Duration};
use tokio::{net::UdpSocket, time::timeout};

// Starting Point for the working thread
pub fn start(fileState: Arc<Mutex<FilesState>>, torrent_file_path: &String) {
    let (torrentParsed, info_hash) = torrent_parser::parse_file(&torrent_file_path);
    fileState.lock().unwrap().name = torrentParsed.info.name.unwrap();
    // Root file to store all the files
    fileState.lock().unwrap().file = Arc::new(Mutex::new(file::File {
        name: String::from("/"),
        file_type: file::FileType::DIRECTORY,
        inner_files: Some(Vec::new()),
        size: 0,
        should_download: true,
    }));

    // Total Torrent Size in Bytes
    let mut totalSize: i64 = 20000;

    File::createFileTree(
        fileState.lock().unwrap().file.clone(),
        torrentParsed.info.files.as_ref().unwrap(),
    );
    let i = fileState.lock().unwrap().file.lock().unwrap().size();
    println!("{:?} ", i);

    const TRANS_ID: i32 = 10;
    const PORT: i16 = 8001;

    let announce_list: &Vec<Vec<String>> = torrentParsed.announce_list.as_ref().unwrap();
    let trackers: Vec<Tracker> = Tracker::getTrackers(&torrentParsed.announce, announce_list);

    let async_block = async move {
        // Communicating Socket Address and UDP Socket
        let socket_address: SocketAddr = format!("[::]:{}", PORT).parse().unwrap();
        let socket = UdpSocket::bind(socket_address).await.unwrap();

        // Iterate over all the trackers
        for tracker in trackers {
            if tracker.protocol == TrackerProtocol::UDP {
                // Create Connection Request
                if let Ok(addrs) = tracker.url.socket_addrs(|| None) {
                    if let Ok(connect_response) =
                        connect_request(TRANS_ID, &socket, &addrs[0]).await
                    {
                        if let Ok(announce_response) = annnounce_request(
                            connect_response.clone(),
                            &socket,
                            &addrs[0],
                            info_hash.clone(),
                        )
                        .await
                        {
                            println!(
                                " Connection Response {:?} | Announce Response : {:?}",
                                connect_response, announce_response
                            );
                        }
                    }
                }
            }
        }
    };

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async_block);
}
