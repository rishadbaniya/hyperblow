#![allow(non_snake_case)]
extern crate serde;
extern crate serde_bencode;
extern crate serde_derive;

mod torrent_details;
mod torrent_parser;
mod ui;

use std::{
    env,
    error::Error,
    sync::{Arc, Mutex},
    thread,
};
use ui::files::FilesState;

//mod percent_encoder;

//pub mod torrent_parser;
//
//use hyper::{body::HttpBody, Client};

//use tokio::io::AsyncWriteExt;
use torrent_details::spit_details;

pub mod work;

use work::start::start as workStart;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// Main thread to work on UI rendering
fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    use ui::ui;
    // Global State that is Shared Across Threads
    let file_state = Arc::new(Mutex::new(FilesState::new()));
    let file_state_working_thread = file_state.clone();
    thread::spawn(move || workStart(file_state_working_thread));

    // Get the argument at index 1 from the CLI command "rtourent xyz.torrent"
    // So that we can get the name of the file i.e xyz.torrentj
    let (torrentParsed, info_hashBytes) = torrent_parser::parse_file(&args[0]);

    if let Some(x) = &torrentParsed.info.files {
        for y in x {
            if let Some(x) = y.path.first() {
                println!("{:?}", x);
            }
        }
    }

    ui::draw_ui(file_state)?;
    Ok(())

    //    let percentEncodedInfoHash = percent_encoder::encode(info_hashBytes);
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
