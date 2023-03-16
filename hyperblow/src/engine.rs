#![allow(dead_code, unused_must_use)]
//// Engine, is the core abstraction over all the state, tasks of the
//// torrent session. It is going to be the backend for the CLI or Desktop Applications.
////
//// Design :
////
//// It's going to support two forms of input.
//// 1. Magnet URI
//// 2. ".torrent" file path
////
//// NOTE : More input methods shall be added if they occur in the future
////
//// Few constraints on this Engine:
////
//// 1. It has its own internal thread(s), runtime, to dowload the torrent.
//// 2. The only abstraction engine is going to share is EngineHandle,
////    which can control core behaviours of engine such as shut it down
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;

use crate::core::TorrentFile;
use std::{sync::Arc, thread::JoinHandle};

#[derive(Debug)]
enum Torrent {
    //MagnetUriTorrent(i32),
    FileTorrent(Arc<TorrentFile>),
}

pub enum TorrentSource {
    //MagnetURI(String),
    FilePath(String),
}

pub struct Engine {
    /// Stores all the torrents that are to be downloaded
    pub torrents: Arc<Mutex<Vec<Arc<TorrentHandle>>>>,

    /// The thread that spawns the tokio runtime, where all the torrents download is gonna take place
    engine_thread_handle: JoinHandle<()>,

    /// An internal sender that sends the newly spawned torrent source from the ui_thread into the engine_thread
    trnt_thread_sender: UnboundedSender<TorrentSource>,

    /// A pointer to the torrent handle spawned by the internal thread is gonna be passed back to the
    trnt_handle_receiver: Arc<Mutex<UnboundedReceiver<Arc<TorrentHandle>>>>,
}

impl Engine {
    /// Creates an instance of the engine
    pub fn new() -> Arc<Self> {
        let torrents = Arc::default();

        // Receivies the torrent source from ui_thread and sends it into the engine thread
        let (tsrc_sd, mut tsrc_rx) = unbounded_channel::<TorrentSource>();

        // Receives the torrent handle from engine thread and sents it back to ui_thread
        let (thdl_sd, thdl_rx) = unbounded_channel::<Arc<TorrentHandle>>();

        let engine_thread_handle = std::thread::spawn(move || {
            let tokio_rt = Self::generate_tokio_runtime();

            tokio_rt.block_on(async move {
                while let Some(src) = tsrc_rx.recv().await {
                    // TODO : Check if there was any error in creating the torrent handle in this
                    // engine_thread and then only run the torrent on the engine thread and send its pointer to the ui_thread
                    let handle = TorrentHandle::new(src).await;
                    let tokio_handle = handle.clone();
                    tokio::task::spawn(async move { tokio_handle.run().await });

                    // Send the handle back to the main thread
                    thdl_sd.send(handle);
                }
            });
        });

        Arc::new(Self {
            torrents,
            engine_thread_handle,
            trnt_thread_sender: tsrc_sd,
            trnt_handle_receiver: Arc::new(Mutex::new(thdl_rx)),
        })
    }

    /// Creates a tokio runtime on thread its called
    fn generate_tokio_runtime() -> Runtime {
        Builder::new_multi_thread().enable_all().build().unwrap()
    }

    /// Takes TorrentSource as input, it sends the TorrentSource to the internal engine_thread and
    /// creates a TorrentHandle from that thread and returns it back to the thread that called this
    /// method i.e ui_thread
    ///
    ///  TODO : Return some verbose error i.e Result<T,G> rather than Option None
    pub async fn spawn(&self, src: TorrentSource) -> Option<Arc<TorrentHandle>> {
        // Sends the torrent source into the engine_thread that holds the tokio runtime
        if let Ok(_) = self.trnt_thread_sender.send(src) {
            let mut torrenthandle_receiver = self.trnt_handle_receiver.lock().await;
            if let Some(handle) = torrenthandle_receiver.recv().await {
                self.torrents.lock().await.push(handle.clone());
                Some(handle)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct TorrentHandle {
    inner: Torrent,
}

impl TorrentHandle {
    /// Consumes the torrent source, may it be a Path or a MagnetURI,
    pub async fn new(src: TorrentSource) -> Arc<TorrentHandle> {
        return match src {
            TorrentSource::FilePath(ref path) => {
                let torrent = TorrentFile::new(path).await.unwrap();
                Arc::new(Self {
                    inner: Torrent::FileTorrent(Arc::new(torrent)),
                })
            }
        };
    }

    pub async fn run(&self) {
        match self.inner {
            //Torrent::MagnetUriTorrent(ref m_torrent) => {}
            Torrent::FileTorrent(ref file_trnt) => {
                file_trnt.run().await;
            }
        }
    }

    /// Gets the name of the torrent blockingly
    pub fn pause_resume(&self) -> String {
        return match self.inner {
            Torrent::FileTorrent(ref file_trnt) => {
                return file_trnt.state.meta_info.info.name.as_ref().unwrap().clone();
            }
            _ => "Testing".to_string(),
        };
    }

    /// Gives the name of the torrent
    pub fn name(&self) -> String {
        return match self.inner {
            Torrent::FileTorrent(ref file_trnt) => {
                if let Some(ref name) = file_trnt.state.meta_info.info.name.clone() {
                    name.clone()
                } else {
                    String::from("Name Not Found!")
                }
            }
        };
    }

    /// Gives the total "bytes" downloaded
    pub fn bytes_complete(&self) -> usize {
        return match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.bytes_complete(),
        };
    }

    /// Gives total size of entire torrent in "bytes"
    pub fn bytes_total(&self) -> usize {
        return match self.inner {
            Torrent::FileTorrent(ref file_trnt) => {
                if let Some(size) = file_trnt.state.meta_info.info.length {
                    size as usize
                } else {
                    0 as usize
                }
            }
        };
    }

    /// Gives total no of pieces
    pub fn pieces_total(&self) -> usize {
        return match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.meta_info.info.pieces.len(),
        };
    }

    /// Gives the total pieces downloaded
    pub fn pieces_downloaded(&self) -> usize {
        return match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.pieces_downloaded(),
        };
    }

    /// Gives the size of torrent piece
    pub fn piece_size(&self) -> usize {
        return match self.inner {
            Torrent::FileTorrent(ref file_trnt) => {
                if let Some(size) = file_trnt.state.meta_info.info.piece_length {
                    size as usize
                } else {
                    0
                }
            }
        };
    }

    /// Gives the total download speed in "bytes/second"
    pub fn download_speed(&self) -> usize {
        // TODO : Add implementation for download speed
        400000
    }

    /// Gives the total upload speed in "bytes/second"
    pub fn upload_speed(&self) -> usize {
        // TODO : Add implementation for upload speed
        100000
    }

    pub fn getFileTree(&self) -> Arc<Mutex<crate::core::File>> {
        return match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.file_tree.as_ref().unwrap().clone(),
        };
    }
}
