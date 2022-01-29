//#![allow(non_snake_case)]
//
//extern crate serde;
//extern crate serde_bencode;
//#[macro_use]
//extern crate serde_derive;
//
//mod percent_encoder;
//mod torrent_details;
//pub mod torrent_parser;
//
//use hyper::{body::HttpBody, Client};
//use std::env;
//use tokio::io::AsyncWriteExt;
//use torrent_details::spit_details;

fn main() {
    //    // Get all arguments passed in the CLI
    //    let args: Vec<String> = env::args().collect();
    //    // Get the argument at index 1 from the CLI command "rtourent xyz.torrent"
    //    // So that we can get the name of the file i.e xyz.torrent
    //    let (torrentParsed, info_hashBytes) = torrent_parser::parse_file(&args[1]);
    //
    //    spit_details(&torrentParsed);
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
