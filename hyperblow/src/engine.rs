use async_trait::async_trait;

use futures::stream::{FuturesUnordered, StreamExt};
use tokio::runtime::Runtime;
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::core::TorrentFile;

use std::{sync::Arc, thread::JoinHandle};

struct Engine {
    /// Stores all the torrents that are to be downloaded
    torrents: Vec<Arc<dyn Torrent>>,

    /// Stores the platform this engine is running in
    pub platform: EnginePlatform,

    /// The thread that spawns the tokio runtime,
    /// where all the torrents download is gonna take place
    engine_thread_handle: JoinHandle<()>,

    /// A internal sender that sends the newly spawned torrent
    /// into the engine_thread
    torrent_sender: UnboundedSender<Arc<dyn Torrent + Send + Sync>>,
}

impl Engine {
    /// Creates an instance of the engine
    fn new() -> Self {
        let platform = EnginePlatform::Unknown;
        let torrents = Vec::new();

        // TODO : Make a versatile runtime, using Builder method
        let engine_runtime = Runtime::new().unwrap();
        let (sd, rx) = unbounded_channel::<Arc<dyn Torrent + Send + Sync>>();
        let torrent_sender = sd;

        let engine_thread_handle = std::thread::spawn(move || {
            let mut torrent_s = FuturesUnordered::new();

            let tokio_runtime = Runtime::new().unwrap();
            tokio_runtime.block_on(async move {
                while let Some(x) = rx.recv().await {
                    let y = x.run();
                    torrent_s.push(y)
                }
                loop {
                    tokio::select! {
                     _ = torrent_s.next() => {}

                    Some(torrent) = rx.recv() => {
                        torrent_s.push(torrent.run());
                      }
                    }
                }
            });
        });

        Self {
            engine_thread_handle,
            platform,
            torrents,
            torrent_sender,
        }
    }

    fn start(&self) {}

    /// Spawns a new torrent to be downloaded and returns
    fn spawn(&self, input: Input) -> () {

        //    match input {
        ////        Input::FilePath(path) => {}
        ////        Input::MagnetURI(uri) => {}
        //    }

        // Check either inpu is a path or a magnet URI, and then decide accordingly either to create
        // a FileTorrent or MagnetURI Torrentsjflsdlsadjflsadjflas
    }
}

/// The platform on which this engine is running
/// If someone is using Hyperblow CLI, then EnginePlatform::CLI will be used
pub enum EnginePlatform {
    CLI,
    Desktop,
    Unknown,
}

//pub enum Torrent {
//    File(FileTorrent),
//    MagnetLink(MagnetLinkTorrent),
//}

pub enum Input {
    MagnetURI(String),
    FilePath(String),
}

///// A trait to perform every CRUD Operations over the state
///// of the Torrent being downloaded
//#[async_trait]
//trait TorrentHandle {
//    /// Starts the downloading and uploading, or resumes if it has been paused
//    fn start(&self);
//
//    /// Temporarily stops the download and upload
//    fn pause(&self);
//}
//
//#[async_trait]
//impl TorrentHandle for TorrentFile {
//    async fn start(&self) {
//        self.run().await;
//    }
//
//    fn pause(&self) {}
//}

#[async_trait]
trait Torrent {
    async fn run(&self);
}

#[async_trait]
impl Torrent for TorrentFile {
    async fn run(&self) {}
}

struct TorrentHandle<T: Torrent> {
    torrent: T,
}

//unsafe impl<T: Torrent> Send for TorrentHandle<T> {}
//
//unsafe impl<T: Torrent> Sync for TorrentHandle<T> {}
//
//impl<T: Torrent> TorrentHandle<T> {
//    fn new(torrent: T ) -> Self {
//        Self { torrent }
//    }
//
//    pub async fn start(&self) {
//
//        //self.torrent.run().await;
//    }
//}
