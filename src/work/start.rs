use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::{BufMut, Bytes, BytesMut};
use crossterm::style::Stylize;
use std::{net::SocketAddr, time::Duration};
use tokio::{io::AsyncReadExt, net::UdpSocket, time::timeout};

// NOTE: When i say files, i mean it in Unix term. Folders are files as well, their type is Directory
use super::{file, torrent_parser};
use crate::{
    ui::files::FilesState,
    work::{
        file::File,
        tracker::{AnnounceRequest, AnnounceResponse, ConnectRequest, TrackerProtocol},
    },
};
use std::sync::{Arc, Mutex};

// Starting Point for the working thread
pub fn start(fileState: Arc<Mutex<FilesState>>, torrent_file_path: &String) {
    // Get the argument at index 1 from the CLI command "rtourent xyz.torrent"
    // So that we can get the name of the file i.e xyz.torrentj
    let (torrentParsed, info_hash) = torrent_parser::parse_file(&torrent_file_path);
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
    let mut totalSize: i64 = 2000;

    File::createFileTree(
        fileState.lock().unwrap().file.clone(),
        torrentParsed.info.files.as_ref().unwrap(),
    );
    let i = fileState.lock().unwrap().file.lock().unwrap().size();
    println!("{:?} ", i);

    use super::tracker::Tracker;

    const TRANSACTION_ID: i32 = 10;
    let announce_list = torrentParsed.announce_list.as_ref().unwrap();
    let trackers: Vec<Tracker> = Tracker::getTrackers(&torrentParsed.announce, announce_list);
    let info_hash1 = info_hash.clone();

    let async_block = async move {
        let localAddr: SocketAddr = "[::]:8001".parse().unwrap();
        let secondLocalAddr: SocketAddr = "[::]:8008".parse().unwrap();
        let socket = UdpSocket::bind(localAddr).await.unwrap();
        let Anothersocket = UdpSocket::bind(secondLocalAddr).await.unwrap();
        for tracker in trackers {
            let mut connect_request = ConnectRequest::empty();
            connect_request.set_transaction_id(TRANSACTION_ID);
            let bytes_to_send = connect_request.getBytesMut();
            if tracker.protocol == TrackerProtocol::UDP {
                match tracker.url.socket_addrs(|| None) {
                    Ok(addrs) => match socket.send_to(&bytes_to_send, addrs[0]).await {
                        Ok(k) => {
                            let mut x = [0u8; 16];
                            if let Ok(_) =
                                timeout(Duration::from_secs(5), socket.recv_from(&mut x)).await
                            {
                                println!(
                                    "Request Has been returned from connect and it is : {:?}",
                                    x
                                );
                                let mut announce = AnnounceRequest::empty();
                                let mut connection_id = &x[8..16];
                                let connection_id =
                                    ReadBytesExt::read_i64::<BigEndian>(&mut connection_id)
                                        .unwrap();
                                announce.set_connection_id(connection_id);
                                announce.set_transaction_id(TRANSACTION_ID);
                                announce.set_info_hash(info_hash1.clone().try_into().unwrap());
                                announce.set_downloaded(0);
                                announce.set_uploaded(0);
                                announce.set_uploaded(0);
                                announce.set_left(totalSize);
                                announce.set_port(8008);
                                announce.set_key(20);
                                let announce_bytes = announce.getBytesMut();
                                if let Ok(s) =
                                    Anothersocket.send_to(&announce_bytes, addrs[0]).await
                                {
                                    let mut yz = vec![0; 512];
                                    if let Ok(v) = timeout(
                                        Duration::from_secs(20),
                                        Anothersocket.recv_from(&mut yz),
                                    )
                                    .await
                                    {
                                        let xx = AnnounceResponse::new(&yz);
                                        println!("Announce Response Received : {:?}", xx);
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
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
