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
use futures::stream::{FuturesUnordered, StreamExt};
use futures::FutureExt;
use tokio::runtime::Runtime;
use tokio::select;
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
            let all_concurrent_tasks = FuturesUnordered::new();
            tokio_runtime.block_on(async move {
                select! {
                    Some(src) = tsrc_rx.recv() => {
                        let handle = TorrentHandle::new(src).await;
                        all_concurrent_tasks.push(handle.run());
                    }

                    _ = all_concurrent_tasks.next() =>{
                    }

                }
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
