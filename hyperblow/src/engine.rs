use std::io::Error;

use futures::stream::FuturesUnordered;
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
//// 3.
use tokio::runtime::Runtime;
//use futures::stream::{FuturesUnordered, StreamExt};
//use futures::FutureExt;
//use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::core::TorrentFile;
use std::{sync::Arc, thread::JoinHandle};

struct Engine {
    /// Stores all the torrents that are to be downloaded
    torrents: Vec<Arc<TorrentHandle>>,
    // Stores the platform this engine is running in
    //pub platform: EnginePlatform,
    /// The thread that spawns the tokio runtime,
    /// where all the torrents download is gonna take place
    engine_thread_handle: JoinHandle<()>,

    /// A internal sender that sends the newly spawned torrent source
    /// into the engine_thread
    torrent_thread_sender: UnboundedSender<TorrentSource>,

    torrenthandle_receiver: UnboundedReceiver<Arc<TorrentHandle>>,
}

impl Engine {
    /// Creates an instance of the engine
    fn new() -> Self {
        let torrents = Vec::new();

        // TODO : Make a versatile runtime, using Builder method
        let engine_runtime = Runtime::new().unwrap();
        let (tsrc_sd, mut tsrc_rx) = unbounded_channel::<TorrentSource>();
        let (thdl_sd, mut thdl_rx) = unbounded_channel::<Arc<TorrentHandle>>();

        let engine_thread_handle = std::thread::spawn(move || {
            let tokio_runtime = Runtime::new().unwrap();
            let futures = FuturesUnordered::new();
            tokio_runtime.block_on(async move {
                tokio::spawn(async move {
                    while let Some(source) = tsrc_rx.recv().await {
                        let handle = TorrentHandle::new(source).await;
                        thdl_sd.send(handle);
                        futures.push(async {});
                    }
                });

                //tokio::spawn(async move { futures.push(async {}) });
            });
        });

        Self {
            torrents,
            engine_thread_handle,
            torrent_thread_sender: tsrc_sd,
            torrenthandle_receiver: thdl_rx,
        }
    }
    //
    //    fn start(&self) {}
    //
    /// Spawns a new TorrentHandle for the torrent to be downloaded and returns
    fn spawn(&mut self, input: TorrentSource) -> Option<Arc<TorrentHandle>> {
        // Sends the torrent source into the engine runtime thread
        self.torrent_thread_sender.send(input);
        // Attempts to receive the TorrentHandle produced from the torrent source
        self.torrenthandle_receiver.blocking_recv()
    }

    // Check either inpu is a path or a magnet URI, and then decide accordingly either to create
    // a FileTorrent or MagnetURI Torrentsjflsdlsadjflsadjflas
}

///// The platform on which this engine is running
///// If someone is using Hyperblow CLI, then EnginePlatform::CLI will be used
//pub enum EnginePlatform {
//    CLI,
//    Desktop,
//    Unknown,
//}
//
////pub enum Torrent {
////    File(FileTorrent),
////    MagnetLink(MagnetLinkTorrent),
////}
//

//
/////// A trait to perform every CRUD Operations over the state
/////// of the Torrent being downloaded
////#[async_trait]
////trait TorrentHandle {
////    /// Starts the downloading and uploading, or resumes if it has been paused
////    fn start(&self);
////
////    /// Temporarily stops the download and upload
////    fn pause(&self);
////}
////
////#[async_trait]
////impl TorrentHandle for TorrentFile {
////    async fn start(&self) {
////        self.run().await;
////    }
////
////    fn pause(&self) {}
////}
//
////unsafe impl<T: Torrent> Send for TorrentHandle<T> {}
////
////unsafe impl<T: Torrent> Sync for TorrentHandle<T> {}
////
////impl<T: Torrent> TorrentHandle<T> {
////    fn new(torrent: T ) -> Self {
////        Self { torrent }
////    }
////
////    pub async fn start(&self) {
////
////        //self.torrent.run().await;
////    }
////}

pub enum TorrentSource {
    //MagnetURI(String),
    FilePath(String),
}

struct TorrentHandle {
    inner: Torrent,
}

impl TorrentHandle {
    /// Consumes the torrent source, may it be a Path or a MagnetURI,
    async fn new(source: TorrentSource) -> Arc<TorrentHandle> {
        return match source {
            TorrentSource::FilePath(ref path) => {
                let torrent = TorrentFile::new(path).await.unwrap();
                Arc::new(Self {
                    inner: Torrent::FileTorrent(torrent),
                })
            }
        };
    }

    async fn run(&self) {
        match self.inner {
            Torrent::MagnetUriTorrent(ref m_torrent) => {}
            Torrent::FileTorrent(ref f_torrent) => {
                f_torrent.run();
            }
        }
    }
}

enum Torrent {
    MagnetUriTorrent(i32),
    FileTorrent(TorrentFile),
}