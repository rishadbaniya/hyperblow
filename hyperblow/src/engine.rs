use std::sync::Arc;

struct Engine {
    /// Stores all the torrents that are to be downloaded
    torrents: Vec<Arc<dyn TorrentHandle>>,

    /// Stores the platform this engine is running in
    platform: EnginePlatform,
}

impl Engine {
    /// Creates an instance of the engine
    fn new() -> Self {
        let platform = EnginePlatform::Unknown;
        let torrents = Vec::new();
        Self { platform, torrents }
    }

    /// Spawns a new torrent to be downloaded and returns
    fn spawn(&self, input: Input) -> Arc<dyn TorrentHandle> {
        //    match input {
        ////        Input::FilePath(path) => {}
        ////        Input::MagnetURI(uri) => {}
        //    }

        // Check either inpu is a path or a magnet URI, and then decide accordingly either to create
        // a FileTorrent or MagnetURI Torrentsjflsdlsadjflsadjflas
        Arc::new(String::from("Test"))
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

/// A trait to perform every CRUD Operations over the state
/// of the Torrent being downloaded
trait TorrentHandle {}

impl TorrentHandle for String {}
