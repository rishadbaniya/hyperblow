use bytes::BufMut;
use crossterm::style::Stylize;
use std::{net::SocketAddr, time::Duration};
use tokio::{net::UdpSocket, time::timeout};

// NOTE: When i say files, i mean it in Unix term. Folders are files as well, their type is Directory
use super::{file, torrent_parser};
use crate::ui::files::FilesState;
use std::sync::{Arc, Mutex};

// Starting Point for the working thread
pub fn start(fileState: Arc<Mutex<FilesState>>, torrent_file_path: &String) {
    // Get the argument at index 1 from the CLI command "rtourent xyz.torrent"
    // So that we can get the name of the file i.e xyz.torrentj
    let (torrentParsed, info_hashBytes) = torrent_parser::parse_file(&torrent_file_path);
    {
        fileState.lock().unwrap().name = torrentParsed.info.name.unwrap();
    }

    // Root file to store all the files
    fileState.lock().unwrap().file = Arc::new(Mutex::new(file::File {
        name: String::from("/"),
        file_type: file::FileType::DIRECTORY,
        inner_files: Some(Vec::new()),
        size: 0,
        should_download: true,
    }));

    // Total Torrent Size in Bytes
    let mut totalSize: i64 = 0;
    if let Some(files) = &torrentParsed.info.files {
        for file in files {
            let mut afile = fileState.lock().unwrap().file.clone();
            for x in 0..file.path.len() {
                let (mut idx, doesContain) = (*afile).lock().unwrap().contains(&file.path[x]);
                if !doesContain {
                    let last_path_index = file.path.len() - 1;
                    totalSize += file.length;
                    idx = Some((*afile).lock().unwrap().add_file(file::File {
                        name: String::from(&file.path[x]),
                        file_type: if x == last_path_index {
                            file::FileType::REGULAR
                        } else {
                            file::FileType::DIRECTORY
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

    let percentEncodedInfoHash = super::percent_encoder::encode(info_hashBytes);
    println!("{} MB", (totalSize / 1024) / 1024);
    println!("{:?}", percentEncodedInfoHash);
    println!("{:?}", torrentParsed.announce_list.as_ref().unwrap());
    println!("{:?}", torrentParsed.announce);

    use super::tracker::Tracker;

    let announce_list = torrentParsed.announce_list.as_ref().unwrap();
    let trackers: Vec<Tracker> = Tracker::getTrackers(&torrentParsed.announce, announce_list);

    let async_block = async move {
        let localAddr: SocketAddr = "[::]:0".parse().unwrap();
        let socket = UdpSocket::bind(localAddr).await.unwrap();
        for tracker in trackers {
            const PROTOCOL_ID: i64 = 0x41727101980;
            const ACTION: i32 = 0;
            let mut bytes_to_send = bytes::BytesMut::with_capacity(16);

            //
            // Offset  Size            Name            Value
            // 0       32-bit integer  action          0 // connect
            // 4       32-bit integer  transaction_id
            // 8       64-bit integer  connection_id
            // 16
            //
            bytes_to_send.put_i64(PROTOCOL_ID);
            bytes_to_send.put_i32(ACTION);
            bytes_to_send.put_i32(10 as i32);
            match tracker.url.socket_addrs(|| None) {
                Ok(addrs) => match socket.send_to(&bytes_to_send, addrs[0]).await {
                    Ok(k) => {
                        let mut x = [0u8; 16];
                        timeout(Duration::from_secs(10), socket.recv_from(&mut x)).await;
                        println!("{:?}", &x)
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    };
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async_block);

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
