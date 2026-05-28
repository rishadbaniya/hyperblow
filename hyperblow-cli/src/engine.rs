#![allow(dead_code)]
/// Engine, is the core abstraction over all the state, tasks of the
/// torrent session. It is going to be the backend for the CLI or Desktop Applications.
///
/// Design :
///
/// It's going to support two forms of input.
/// 1. Magnet URI
/// 2. ".torrent" file path
///
/// NOTE : More input methods shall be added if they occur in the future
///
/// Few constraints on this Engine:
///
/// 1. It has its own internal thread(s), runtime, to dowload the torrent.
/// 2. The only abstraction engine is going to share is EngineHandle,
///    which can control core behaviours of engine such as shut it down
use crate::core::{tracker::TrackerState, TorrentFile};
use hyperblow::parser::magnet_uri_parser::MagnetURIMeta;
use std::{sync::Arc, thread::JoinHandle};
use thiserror::Error;
use tokio::{
    runtime::{Builder, Runtime},
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};

#[derive(Debug)]
enum Torrent {
    MagnetUriTorrent(Arc<MagnetURIMeta>),
    FileTorrent(Arc<TorrentFile>),
}

pub enum TorrentSource {
    MagnetURI(String),
    FilePath(String),
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("engine command channel closed")]
    CommandChannelClosed,

    #[error("engine response channel closed")]
    ResponseChannelClosed,

    #[error("invalid torrent file: {0}")]
    InvalidTorrentFile(String),

    #[error("invalid magnet URI")]
    InvalidMagnetUri,
}

pub struct TrackerSnapshot {
    pub url: String,
    pub status: String,
    pub is_error: bool,
}

type TorrentHandleResultReceiver = Arc<Mutex<UnboundedReceiver<Result<Arc<TorrentHandle>, EngineError>>>>;

pub struct Engine {
    /// Stores all the torrents that are to be downloaded
    pub torrents: Arc<Mutex<Vec<Arc<TorrentHandle>>>>,

    /// The thread that spawns the tokio runtime, where all the torrents download is gonna take place
    engine_thread_handle: JoinHandle<()>,

    /// An internal sender that sends the newly spawned torrent source from the ui_thread into the engine_thread
    trnt_thread_sender: UnboundedSender<TorrentSource>,

    /// A pointer to the torrent handle spawned by the internal thread is gonna be passed back to the
    trnt_handle_receiver: TorrentHandleResultReceiver,
}

impl Engine {
    /// Creates an instance of the engine
    pub fn new() -> Arc<Self> {
        let torrents = Arc::default();

        // Receivies the torrent source from ui_thread and sends it into the engine thread
        let (tsrc_sd, mut tsrc_rx) = unbounded_channel::<TorrentSource>();

        // Receives the torrent handle from engine thread and sents it back to ui_thread
        let (thdl_sd, thdl_rx) = unbounded_channel::<Result<Arc<TorrentHandle>, EngineError>>();

        let engine_thread_handle = std::thread::spawn(move || {
            let tokio_rt = Self::generate_tokio_runtime();

            tokio_rt.block_on(async move {
                while let Some(src) = tsrc_rx.recv().await {
                    // TODO : Check if there was any error in creating the torrent handle in this
                    // engine_thread and then only run the torrent on the engine thread and send its pointer to the ui_thread
                    let handle = TorrentHandle::new(src).await;
                    if let Ok(ref handle) = handle {
                        let tokio_handle = handle.clone();
                        tokio::task::spawn(async move { tokio_handle.run().await });
                    }

                    // Send the handle back to the main thread
                    if thdl_sd.send(handle).is_err() {
                        break;
                    }
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
    pub async fn spawn(&self, src: TorrentSource) -> Result<Arc<TorrentHandle>, EngineError> {
        // Sends the torrent source into the engine_thread that holds the tokio runtime
        self.trnt_thread_sender.send(src).map_err(|_| EngineError::CommandChannelClosed)?;
        let mut torrenthandle_receiver = self.trnt_handle_receiver.lock().await;
        let handle = torrenthandle_receiver.recv().await.ok_or(EngineError::ResponseChannelClosed)??;
        self.torrents.lock().await.push(handle.clone());
        Ok(handle)
    }
}

#[derive(Debug)]
pub struct TorrentHandle {
    inner: Torrent,
}

impl TorrentHandle {
    /// Consumes the torrent source, may it be a Path or a MagnetURI,
    pub async fn new(src: TorrentSource) -> Result<Arc<TorrentHandle>, EngineError> {
        match src {
            TorrentSource::FilePath(ref path) => {
                let torrent = TorrentFile::new(path)
                    .await
                    .map_err(|error| EngineError::InvalidTorrentFile(format!("{error:?}")))?;
                Ok(Arc::new(Self {
                    inner: Torrent::FileTorrent(Arc::new(torrent)),
                }))
            }
            TorrentSource::MagnetURI(ref uri) => {
                let magnet = MagnetURIMeta::fromMagnetURI(uri).map_err(|_| EngineError::InvalidMagnetUri)?;
                Ok(Arc::new(Self {
                    inner: Torrent::MagnetUriTorrent(Arc::new(magnet)),
                }))
            }
        }
    }

    pub async fn run(&self) {
        match self.inner {
            Torrent::MagnetUriTorrent(_) => {}
            Torrent::FileTorrent(ref file_trnt) => {
                file_trnt.run().await;
            }
        }
    }

    /// Gets the name of the torrent blockingly
    pub fn pause_resume(&self) -> String {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt
                .state
                .meta_info
                .info
                .name
                .clone()
                .unwrap_or_else(|| "Unnamed torrent".to_string()),
            Torrent::MagnetUriTorrent(ref magnet) => magnet.dn.clone().unwrap_or_else(|| "Magnet torrent".to_string()),
        }
    }

    /// Gives the name of the torrent
    pub fn name(&self) -> String {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => {
                if let Some(ref name) = file_trnt.state.meta_info.info.name.clone() {
                    name.clone()
                } else {
                    String::from("Name Not Found!")
                }
            }
            Torrent::MagnetUriTorrent(ref magnet) => magnet
                .dn
                .clone()
                .or_else(|| magnet.xt.clone())
                .unwrap_or_else(|| "Magnet torrent".to_string()),
        }
    }

    /// Gives the total "bytes" downloaded
    pub fn bytes_complete(&self) -> usize {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.bytes_complete(),
            Torrent::MagnetUriTorrent(_) => 0,
        }
    }

    /// Gives total size of entire torrent in "bytes"
    pub fn bytes_total(&self) -> usize {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.meta_info.total_length().max(0) as usize,
            Torrent::MagnetUriTorrent(ref magnet) => magnet.xl.unwrap_or(0) as usize,
        }
    }

    /// Gives total no of pieces
    pub fn pieces_total(&self) -> usize {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.meta_info.piece_count(),
            Torrent::MagnetUriTorrent(_) => 0,
        }
    }

    /// Gives the total pieces downloaded
    pub fn pieces_downloaded(&self) -> usize {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.pieces_downloaded(),
            Torrent::MagnetUriTorrent(_) => 0,
        }
    }

    /// Gives the size of torrent piece
    pub fn piece_size(&self) -> usize {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => {
                if let Some(size) = file_trnt.state.meta_info.info.piece_length {
                    size as usize
                } else {
                    0
                }
            }
            Torrent::MagnetUriTorrent(_) => 0,
        }
    }

    /// Gives the total download speed in "bytes/second".
    pub fn download_speed(&self) -> usize {
        0
    }

    /// Gives the total upload speed in "bytes/second".
    pub fn upload_speed(&self) -> usize {
        0
    }

    pub fn status_label(&self) -> String {
        match self.inner {
            Torrent::FileTorrent(_) => "Preparing".to_string(),
            Torrent::MagnetUriTorrent(_) => "Metadata only".to_string(),
        }
    }

    pub fn getFileTree(&self) -> Option<Arc<Mutex<crate::core::File>>> {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.file_tree.clone(),
            Torrent::MagnetUriTorrent(_) => None,
        }
    }

    pub fn tracker_snapshots(&self) -> Vec<TrackerSnapshot> {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => {
                let trackers = file_trnt.state.trackers.blocking_read();
                trackers
                    .iter()
                    .flat_map(|tier| tier.iter())
                    .map(|tracker| {
                        let status = tracker.tracker_state.load();
                        TrackerSnapshot {
                            url: tracker.address.to_string(),
                            status: status.to_string(),
                            is_error: matches!(status, TrackerState::DNSUnresolved { .. } | TrackerState::Idle),
                        }
                    })
                    .collect()
            }
            Torrent::MagnetUriTorrent(ref magnet) => magnet
                .tr
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(|url| TrackerSnapshot {
                    url,
                    status: "Magnet metadata".to_string(),
                    is_error: false,
                })
                .collect(),
        }
    }

    pub fn connected_peers(&self) -> usize {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.peers.blocking_lock().len(),
            Torrent::MagnetUriTorrent(_) => 0,
        }
    }

    pub fn peer_addresses(&self) -> Vec<String> {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt
                .state
                .peers
                .blocking_lock()
                .iter()
                .map(|peer| peer.socket_adr.to_string())
                .collect(),
            Torrent::MagnetUriTorrent(_) => Vec::new(),
        }
    }

    pub fn file_tree_names(&self) -> Vec<String> {
        match self.getFileTree() {
            Some(file_tree) => file_tree.blocking_lock().tabs_traverse_names_blocking(0),
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Engine, TorrentSource};

    #[tokio::test]
    async fn spawns_magnet_handle_without_network_setup() {
        let engine = Engine::new();
        let handle = engine
            .spawn(TorrentSource::MagnetURI(
                "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10&dn=Sintel&tr=udp://tracker.example.com:6969".to_string(),
            ))
            .await
            .expect("magnet should spawn");

        assert_eq!(handle.name(), "Sintel");
        assert_eq!(handle.tracker_snapshots().len(), 1);
        assert_eq!(engine.torrents.lock().await.len(), 1);
    }
}
