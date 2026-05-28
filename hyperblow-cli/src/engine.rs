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
use crate::{
    core::{
        magnet::{MagnetTorrent, MagnetTorrentError},
        state::State,
        tracker::TrackerState,
        TError, TorrentFile,
    },
    download_directory::{DownloadDirectory, DownloadDirectoryError},
};
use hyperblow::parser::magnet_uri_parser::MagnetURIMeta;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread::JoinHandle,
};
use thiserror::Error;
use tokio::{
    runtime::{Builder, Runtime},
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};
use tracing::{debug, error, info, warn};

#[derive(Debug)]
enum Torrent {
    MagnetUriTorrent(Arc<MagnetTorrent>),
    FileTorrent(Arc<TorrentFile>),
}

pub enum TorrentSource {
    MagnetURI(String),
    FilePath(String),
}

impl TorrentSource {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::MagnetURI(_) => "magnet",
            Self::FilePath(_) => "file",
        }
    }
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("engine command channel closed")]
    CommandChannelClosed,

    #[error("engine response channel closed")]
    ResponseChannelClosed,

    #[error("invalid torrent file")]
    InvalidTorrentFile(#[from] TError),

    #[error("invalid magnet URI")]
    InvalidMagnetUri,

    #[error(transparent)]
    Magnet(#[from] MagnetTorrentError),

    #[error("invalid download directory")]
    DownloadDirectory(#[from] DownloadDirectoryError),
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

    download_directory: DownloadDirectory,

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
        Self::try_new().expect("default download directory should be usable")
    }

    pub fn try_new() -> Result<Arc<Self>, EngineError> {
        let download_directory = DownloadDirectory::default()?;
        download_directory.ensure_exists()?;
        Ok(Self::with_download_directory(download_directory))
    }

    fn with_download_directory(download_directory: DownloadDirectory) -> Arc<Self> {
        let torrents = Arc::default();
        let engine_download_directory = download_directory.clone();
        debug!(download_directory = %download_directory.path().display(), "creating engine");

        // Receivies the torrent source from ui_thread and sends it into the engine thread
        let (tsrc_sd, mut tsrc_rx) = unbounded_channel::<TorrentSource>();

        // Receives the torrent handle from engine thread and sents it back to ui_thread
        let (thdl_sd, thdl_rx) = unbounded_channel::<Result<Arc<TorrentHandle>, EngineError>>();

        let engine_thread_handle = std::thread::spawn(move || {
            let tokio_rt = Self::generate_tokio_runtime();

            tokio_rt.block_on(async move {
                while let Some(src) = tsrc_rx.recv().await {
                    let source_kind = src.kind();
                    debug!(source = source_kind, "engine received torrent source");
                    // TODO : Check if there was any error in creating the torrent handle in this
                    // engine_thread and then only run the torrent on the engine thread and send its pointer to the ui_thread
                    let handle = TorrentHandle::new(src, engine_download_directory.path().to_path_buf()).await;
                    match &handle {
                        Ok(handle) => {
                            info!(source = source_kind, torrent = %handle.name(), "torrent handle created");
                            let tokio_handle = handle.clone();
                            tokio::task::spawn(async move { tokio_handle.run().await });
                        }
                        Err(error) => {
                            error!(source = source_kind, error = %error, "failed to create torrent handle");
                        }
                    }

                    // Send the handle back to the main thread
                    if thdl_sd.send(handle).is_err() {
                        warn!(source = source_kind, "engine response receiver closed");
                        break;
                    }
                }
            });
        });

        Arc::new(Self {
            torrents,
            download_directory,
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
        let source_kind = src.kind();
        debug!(source = source_kind, "queueing torrent spawn");
        // Sends the torrent source into the engine_thread that holds the tokio runtime
        self.trnt_thread_sender.send(src).map_err(|_| EngineError::CommandChannelClosed)?;
        let mut torrenthandle_receiver = self.trnt_handle_receiver.lock().await;
        let handle = torrenthandle_receiver.recv().await.ok_or(EngineError::ResponseChannelClosed)??;
        info!(source = source_kind, torrent = %handle.name(), "torrent spawned");
        self.torrents.lock().await.push(handle.clone());
        Ok(handle)
    }

    pub fn torrent_snapshot(&self) -> Option<Vec<Arc<TorrentHandle>>> {
        self.torrents.try_lock().ok().map(|handles| handles.clone())
    }

    pub fn download_directory(&self) -> &Path {
        self.download_directory.path()
    }
}

#[derive(Debug)]
pub struct TorrentHandle {
    inner: Torrent,
    download_directory: PathBuf,
}

impl TorrentHandle {
    /// Consumes the torrent source, may it be a Path or a MagnetURI,
    pub async fn new(src: TorrentSource, download_directory: PathBuf) -> Result<Arc<TorrentHandle>, EngineError> {
        debug!(source = src.kind(), download_directory = %download_directory.display(), "building torrent handle");
        match src {
            TorrentSource::FilePath(ref path) => {
                debug!(source = "file", path = %path, "loading torrent file");
                let torrent = TorrentFile::new(path, download_directory.clone()).await?;
                Ok(Arc::new(Self {
                    inner: Torrent::FileTorrent(Arc::new(torrent)),
                    download_directory,
                }))
            }
            TorrentSource::MagnetURI(ref uri) => {
                debug!(source = "magnet", "parsing magnet URI");
                let magnet = MagnetURIMeta::fromMagnetURI(uri).map_err(|_| EngineError::InvalidMagnetUri)?;
                let magnet = MagnetTorrent::new(magnet, download_directory.clone()).await?;
                Ok(Arc::new(Self {
                    inner: Torrent::MagnetUriTorrent(Arc::new(magnet)),
                    download_directory,
                }))
            }
        }
    }

    pub async fn run(&self) {
        match self.inner {
            Torrent::MagnetUriTorrent(ref magnet) => {
                magnet.run().await;
            }
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
            Torrent::MagnetUriTorrent(ref magnet) => MagnetTitle::from_meta(magnet.meta()).display(),
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
            Torrent::MagnetUriTorrent(ref magnet) => MagnetTitle::from_meta(magnet.meta()).display(),
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
        self.bytes_total_known().unwrap_or(0)
    }

    pub fn bytes_total_known(&self) -> Option<usize> {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => Some(file_trnt.state.meta_info.total_length().max(0) as usize),
            Torrent::MagnetUriTorrent(ref magnet) => magnet.bytes_total(),
        }
    }

    pub fn progress_percent(&self) -> u16 {
        let Some(bytes_total) = self.bytes_total_known() else {
            return 0;
        };
        if bytes_total == 0 {
            return 0;
        }

        ((self.bytes_complete().saturating_mul(100)) / bytes_total).min(100) as u16
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
            Torrent::MagnetUriTorrent(ref magnet) => magnet.status_label(),
        }
    }

    pub fn download_directory(&self) -> &Path {
        &self.download_directory
    }

    pub fn getFileTree(&self) -> Option<Arc<Mutex<crate::core::File>>> {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.file_tree.clone(),
            Torrent::MagnetUriTorrent(_) => None,
        }
    }

    pub fn tracker_snapshots(&self) -> Vec<TrackerSnapshot> {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => TrackerSnapshotList::from_state(&file_trnt.state, Vec::new()),
            Torrent::MagnetUriTorrent(ref magnet) => TrackerSnapshotList::from_state(&magnet.state(), magnet.tracker_addresses()),
        }
    }

    pub fn connected_peers(&self) -> usize {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.peers.try_lock().map(|peers| peers.len()).unwrap_or_default(),
            Torrent::MagnetUriTorrent(ref magnet) => magnet.state().peers.try_lock().map(|peers| peers.len()).unwrap_or_default(),
        }
    }

    pub fn peer_addresses(&self) -> Vec<String> {
        match self.inner {
            Torrent::FileTorrent(ref file_trnt) => file_trnt.state.peers.try_lock().map_or_else(
                |_| Vec::new(),
                |peers| peers.iter().map(|peer| peer.socket_adr.to_string()).collect(),
            ),
            Torrent::MagnetUriTorrent(ref magnet) => magnet.state().peers.try_lock().map_or_else(
                |_| Vec::new(),
                |peers| peers.iter().map(|peer| peer.socket_adr.to_string()).collect(),
            ),
        }
    }

    pub fn file_tree_names(&self) -> Vec<String> {
        match self.getFileTree() {
            Some(file_tree) => file_tree
                .try_lock()
                .map_or_else(|_| Vec::new(), |file_tree| file_tree.try_tabs_traverse_names(0)),
            None => Vec::new(),
        }
    }
}

struct TrackerSnapshotList;

impl TrackerSnapshotList {
    fn from_state(state: &Arc<State>, fallback_addresses: Vec<String>) -> Vec<TrackerSnapshot> {
        let Ok(trackers) = state.trackers.try_read() else {
            return Self::queued(fallback_addresses);
        };

        let snapshots = trackers
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
            .collect::<Vec<_>>();

        if snapshots.is_empty() {
            Self::queued(fallback_addresses)
        } else {
            snapshots
        }
    }

    fn queued(addresses: Vec<String>) -> Vec<TrackerSnapshot> {
        addresses
            .into_iter()
            .map(|url| TrackerSnapshot {
                url,
                status: "Queued".to_string(),
                is_error: false,
            })
            .collect()
    }
}

struct MagnetTitle {
    display: String,
}

impl MagnetTitle {
    fn from_meta(meta: &MagnetURIMeta) -> Self {
        if let Some(display_name) = meta.dn.as_deref().map(str::trim).filter(|name| !name.is_empty()) {
            return Self {
                display: display_name.to_string(),
            };
        }

        if let Some(hash) = meta.xt.as_deref().and_then(Self::hash_from_exact_topic) {
            return Self {
                display: format!("Magnet {}", Self::short_hash(hash)),
            };
        }

        Self {
            display: "Magnet torrent".to_string(),
        }
    }

    fn display(self) -> String {
        self.display
    }

    fn hash_from_exact_topic(exact_topic: &str) -> Option<&str> {
        exact_topic.rsplit_once(':').map(|(_, hash)| hash).filter(|hash| !hash.is_empty())
    }

    fn short_hash(hash: &str) -> String {
        hash.chars().take(12).collect()
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
        assert_eq!(handle.status_label(), "Fetching metadata");
        assert_eq!(handle.bytes_total_known(), None);
        assert_eq!(handle.download_directory(), engine.download_directory());
        assert!(engine.download_directory().is_dir());
        assert_eq!(handle.tracker_snapshots().len(), 1);
        assert_eq!(engine.torrents.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn magnet_title_is_readable_without_display_name() {
        let engine = Engine::new();
        let handle = engine
            .spawn(TorrentSource::MagnetURI(
                "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10".to_string(),
            ))
            .await
            .expect("magnet should spawn");

        assert_eq!(handle.name(), "Magnet 08ada5a7a618");
    }
}
