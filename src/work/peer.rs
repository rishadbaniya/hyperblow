#![allow(non_camel_case_types)]
#![allow(unused_must_use)]
use crate::work::message::{self, Interested, Unchoke};

use super::message::{Bitfield, Extended, Handshake, Have};
use super::start::__Details;
use bytes::{BufMut, BytesMut};
use futures::{select, FutureExt};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::WriteHalf;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

// PEER REQUEST (TCP) :
//
// OBJECTIVE : Connect to Peers and download pieces(blocks)
// First of all, we make a TCP connection with the "peer"
pub async fn peer_request(socket_adr: SocketAddr, details: __Details) {
    const CONNECTION_TIMEOUT: u64 = 60;

    loop {
        // Attempts to make a TCP connection with the peer until CONNECTION_TIMEOUT has passed
        match timeout(Duration::from_secs(CONNECTION_TIMEOUT), TcpStream::connect(socket_adr)).await {
            // Means TCP connection was established
            Ok(v) => match v {
                Ok(mut stream) => {
                    // Split the TCP stream into read and write half
                    let (mut read_half, mut write_half) = stream.split();
                    // Channelt to communicate data between read and write half :
                    // TODO: Find perfect channel buffer size, currently its set almost same size as TCP window size
                    let (sender, mut receiver) = mpsc::channel::<BytesMut>(70000);

                    // READ HALF :
                    // Continuosly reads on the TCP stream until EOF
                    let read = async move {
                        'read_loop: loop {
                            let mut buf = BytesMut::with_capacity(70000);
                            match read_half.read_buf(&mut buf).await {
                                Ok(v) => {
                                    if v == 0 {
                                        break 'read_loop;
                                    }
                                    sender.send(buf).await.unwrap();
                                }
                                Err(e) => println!("{:?}", e),
                            }
                        }
                    };

                    // WRITE HALF :
                    //
                    // NOTE :According to my knowledge, TCP is not req-res like most protocols built on top of TCP, HTTP is req-res not because its
                    // built on top TCP
                    //
                    // TCP is bidirectional, so once we have sent a Handshake message, we
                    // can wait until some message has arrived on the Socket, if the messages
                    // has arrived then we read it and deserialize it to certain Message Types
                    let write_details = details.clone();
                    let write = async move {
                        let write_details = write_details;
                        let mut messages: Vec<Message> = Vec::new();
                        let mut handshake_msg = Handshake::empty();
                        handshake_msg.set_info_hash(write_details.lock().await.info_hash.as_ref().unwrap().clone());

                        // Writes "Handshake message" on the TCP stream
                        write_half.write_all(&handshake_msg.getBytesMut()).await.unwrap();

                        // NOTE : This block must run once
                        //
                        // First phase i.e peer has sent responses for the "Handshake" message we sent
                        // It gets all the message that peer has sent as a response and
                        // deserializes it, it even handles incosistency among message, like
                        // mulitple messages sent by peer as one single and multiple messages sent
                        // in different packet
                        if let Some(msg) = receiver.recv().await {
                            // If messages is empty, then it means we were waiting for some message
                            // to come after a Handshake request was sent
                            if messages.is_empty() {
                                // We'll push all the bytes sent to us by the peer as a response of the "Handshake" message we sent into one big chunk
                                // i.e "response_from_handshake" and then deserialize all messages out of this big chunk
                                let mut response_from_handshake = BytesMut::new();
                                response_from_handshake.put_slice(&msg);
                                timeout(Duration::from_secs(2), async {
                                    loop {
                                        if let Some(v) = receiver.recv().await {
                                            response_from_handshake.put_slice(&v);
                                        }
                                    }
                                })
                                .await;
                                messages.append(&mut handshake_responses(&mut response_from_handshake));
                            }
                        }

                        // On Choke, shutdown the TCP stream and stop progression of the future
                        if messages.contains(&Message::CHOKE) {
                            write_half.shutdown();
                            return;
                        }

                        // Send interested message to the peers and unchoke expect them to send unchoke message
                        write_half.write_all(&Interested::build_message()).await.unwrap();

                        if let Some(mut msg) = receiver.recv().await {
                            messages.push(messageHandler(&mut msg).unwrap());
                        }

                        // On Choke, shutdown the TCP stream and stop progression of the future
                        if messages.contains(&Message::CHOKE) {
                            write_half.shutdown();
                            return;
                        }

                        if messages.contains(&Message::UNCHOKE) {
                            write_half.write_all(&Unchoke::build_message()).await.unwrap();
                        }
                    };

                    // End both the future as soon as one gets completed
                    select! {
                        () = read.fuse() => (),
                        () = write.fuse() => ()
                    };
                }
                Err(_) => {
                    // Connection Refused or Something related with Socket address
                    sleep(Duration::from_secs(240)).await
                }
            },
            Err(_) => {
                // Timeout Error
                sleep(Duration::from_secs(240)).await
            }
        }
        sleep(Duration::from_secs(100)).await;
    }
}

fn handshake_responses(bytes: &mut BytesMut) -> Vec<Message> {
    let mut messages: Vec<Message> = Vec::new();
    while let Some(msg) = messageHandler(bytes) {
        messages.push(msg);
    }
    messages
}

// NOTE : Digests bytes acoording to the message found
// This function removes bytes from buffer that is provided after it founds a message
fn messageHandler(bytes: &mut BytesMut) -> Option<Message> {
    if bytes.len() == 0 {
        // If the buffer is empty then it means there is no message
        None
    } else if bytes.len() == 4 {
        // TODO : Check if the length is (0_u32) as well
        Some(Message::KEEP_ALIVE)
    } else {
        // If it's a HANDSHAKE message, then the first message is pstr_len, whose value is 19
        // TODO : Check if pstr = "BitTorrent protocol" as well
        let pstr_len = bytes[0];

        if pstr_len == 19u8 {
            let handshake_msg = Handshake::from(&bytes.split_to(68));
            Some(Message::HANDSHAKE(handshake_msg))
        } else {
            let mut message_id = 100;
            if let Some(v) = bytes.get(4) {
                message_id = *v;
            }
            match message_id {
                0 => {
                    bytes.split_to(5);
                    Some(Message::CHOKE)
                }
                1 => {
                    bytes.split_to(5);
                    Some(Message::UNCHOKE)
                }
                2 => {
                    bytes.split_to(5);
                    Some(Message::INTERESTED)
                }
                3 => {
                    bytes.split_to(5);
                    Some(Message::NOT_INTERESTED)
                }
                4 => Some(Message::HAVE(Have::from(bytes))),
                5 => Some(Message::BITFIELD(Bitfield::from(bytes))),
                6 => Some(Message::REQUEST),
                7 => Some(Message::PIECE),
                8 => Some(Message::CANCEL),
                9 => Some(Message::PORT),
                20 => Some(Message::EXTENDED(Extended::from(bytes))),
                _ => None,
            }
        }
    }
}

//
// Messages sent to the peer and recieved form the peer takes the following forms
//
// All possible messages are specified by BitTorrent Specifications at :
//
// https://wiki.theory.org/index.php/BitTorrentSpecification#Messages
// https://www.bittorrent.org/beps/bep_0010.html
//
#[derive(PartialEq, Debug, Clone)]
enum Message {
    HANDSHAKE(Handshake),
    BITFIELD(Bitfield),
    EXTENDED(Extended),
    HAVE(Have),
    KEEP_ALIVE,
    CHOKE,
    UNCHOKE,
    INTERESTED,
    NOT_INTERESTED,
    REQUEST,
    PIECE,
    CANCEL,
    PORT,
}
