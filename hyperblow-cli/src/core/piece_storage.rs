use super::state::State;
use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{
    fs::{create_dir_all, OpenOptions},
    io::{AsyncSeekExt, AsyncWriteExt, SeekFrom},
};

#[derive(Debug, Error)]
pub enum PieceStorageError {
    #[error("torrent metadata does not define piece length")]
    MissingPieceLength,

    #[error("piece index {piece_index} has no output range")]
    PieceOutOfRange { piece_index: usize },

    #[error("piece data was longer than mapped files")]
    PieceDataTooLong,

    #[error("file storage error")]
    Io(#[from] std::io::Error),
}

pub struct PieceStorage;

impl PieceStorage {
    pub async fn write_piece(state: &Arc<State>, piece_index: usize, piece: &[u8]) -> Result<(), PieceStorageError> {
        let piece_length = state.piece_length().ok_or(PieceStorageError::MissingPieceLength)?;
        let piece_offset = piece_index.saturating_mul(piece_length);
        let files = TorrentOutputFiles::from_state(state);
        let mut remaining = piece;
        let mut cursor = piece_offset;

        for file in files {
            if remaining.is_empty() {
                break;
            }
            if cursor >= file.end_offset() {
                continue;
            }
            if cursor < file.start_offset {
                break;
            }

            let offset_in_file = cursor - file.start_offset;
            let writable = remaining.len().min(file.length.saturating_sub(offset_in_file));
            if writable == 0 {
                continue;
            }

            FileSliceWriter::write(&file.path, offset_in_file as u64, &remaining[..writable]).await?;
            remaining = &remaining[writable..];
            cursor = cursor.saturating_add(writable);
        }

        if remaining.is_empty() {
            Ok(())
        } else if cursor == piece_offset {
            Err(PieceStorageError::PieceOutOfRange { piece_index })
        } else {
            Err(PieceStorageError::PieceDataTooLong)
        }
    }
}

struct TorrentOutputFiles;

impl TorrentOutputFiles {
    fn from_state(state: &State) -> Vec<OutputFile> {
        let mut files = Vec::new();
        let root_name = state.meta_info.info.name.as_deref().unwrap_or("download");
        if let Some(file_entries) = state.meta_info.info.files.as_ref() {
            let root_path = SafePath::join(state.download_directory.clone(), root_name);
            let mut start_offset = 0_usize;
            for file in file_entries {
                let path = file
                    .path
                    .iter()
                    .fold(root_path.clone(), |path, component| SafePath::join(path, component));
                let length = file.length.max(0) as usize;
                files.push(OutputFile {
                    path,
                    start_offset,
                    length,
                });
                start_offset = start_offset.saturating_add(length);
            }
        } else {
            files.push(OutputFile {
                path: SafePath::join(state.download_directory.clone(), root_name),
                start_offset: 0,
                length: state.meta_info.total_length().max(0) as usize,
            });
        }
        files
    }
}

struct OutputFile {
    path: PathBuf,
    start_offset: usize,
    length: usize,
}

impl OutputFile {
    fn end_offset(&self) -> usize {
        self.start_offset.saturating_add(self.length)
    }
}

struct FileSliceWriter;

impl FileSliceWriter {
    async fn write(path: &Path, offset: u64, bytes: &[u8]) -> Result<(), PieceStorageError> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }
        let mut file = OpenOptions::new().create(true).write(true).truncate(false).open(path).await?;
        file.seek(SeekFrom::Start(offset)).await?;
        file.write_all(bytes).await?;
        file.flush().await?;
        Ok(())
    }
}

struct SafePath;

impl SafePath {
    fn join(base: PathBuf, component: &str) -> PathBuf {
        let candidate = Path::new(component);
        if candidate.components().all(|component| matches!(component, Component::Normal(_))) {
            base.join(candidate)
        } else {
            base.join("unsafe_path_component")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PieceStorage;
    use crate::core::{
        piece_picker::PiecePicker,
        state::{DownState, State},
    };
    use crossbeam::atomic::AtomicCell;
    use hyperblow::parser::torrent_parser::{File, FileMeta, Info};
    use sha1::{Digest, Sha1};
    use std::{fs, path::PathBuf, sync::Arc};
    use tokio::sync::{Mutex, RwLock};

    #[tokio::test]
    async fn writes_single_file_piece_to_download_directory() {
        let output_dir = TestOutput::temp_dir();
        let piece = b"hello".to_vec();
        let state = TestOutput::state(output_dir.clone(), piece.clone());

        PieceStorage::write_piece(&state, 0, &piece).await.expect("piece should write");

        assert_eq!(fs::read(output_dir.join("piece-test.bin")).expect("output should exist"), piece);
        fs::remove_dir_all(output_dir).expect("output dir should remove");
    }

    #[tokio::test]
    async fn writes_multi_file_piece_without_trusting_metadata_paths() {
        let output_dir = TestOutput::temp_dir();
        let piece = b"safe".to_vec();
        let unique = TestOutput::unique_name();
        let escaped_root = format!("../hyperblow-piece-storage-escaped-root-{unique}");
        let escaped_file = format!("hyperblow-piece-storage-escaped-file-{unique}.bin");
        let mut state = TestOutput::state(output_dir.clone(), piece.clone());
        let inner = Arc::get_mut(&mut state).expect("state should be uniquely owned");
        inner.meta_info.info.name = Some(escaped_root);
        inner.meta_info.info.length = None;
        inner.meta_info.info.files = Some(vec![File {
            length: piece.len() as i64,
            path: vec!["..".to_string(), escaped_file.clone()],
            md5sum: None,
        }]);

        PieceStorage::write_piece(&state, 0, &piece).await.expect("piece should write");

        assert_eq!(
            fs::read(
                output_dir
                    .join("unsafe_path_component")
                    .join("unsafe_path_component")
                    .join(&escaped_file)
            )
            .expect("safe output should exist"),
            piece
        );
        assert!(!output_dir.join(&escaped_file).exists());
        assert!(!output_dir.parent().expect("output dir has parent").join(&escaped_file).exists());
        fs::remove_dir_all(output_dir).expect("output dir should remove");
    }

    struct TestOutput;

    impl TestOutput {
        fn temp_dir() -> PathBuf {
            let path = std::env::temp_dir().join(format!("hyperblow-piece-storage-{}", Self::unique_name()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("temp dir should create");
            path
        }

        fn unique_name() -> String {
            format!("{}-{:?}", std::process::id(), std::thread::current().id())
        }

        fn state(download_directory: PathBuf, piece: Vec<u8>) -> Arc<State> {
            let hash: [u8; 20] = Sha1::digest(&piece).into();
            Arc::new(State {
                meta_info: FileMeta {
                    announce: "udp://tracker.example.test:6969".to_string(),
                    announce_list: None,
                    info: Info {
                        name: Some("piece-test.bin".to_string()),
                        length: Some(piece.len() as i64),
                        files: None,
                        piece_length: Some(piece.len() as i64),
                        pieces: hash.to_vec(),
                    },
                    creation_data: None,
                    comment: None,
                    encoding: None,
                    created_by: None,
                    acceptable_source: None,
                },
                download_directory,
                d_state: DownState::Unknown,
                file_tree: None,
                trackers: Arc::new(RwLock::new(Vec::new())),
                udp_ports: Arc::new(Mutex::new(Vec::new())),
                tcp_ports: Arc::new(Mutex::new(Vec::new())),
                info_hash: vec![1; 20],
                pieces_hash: vec![hash],
                piece_picker: Arc::new(Mutex::new(PiecePicker::new(1))),
                peers: Arc::new(Mutex::new(Vec::new())),
                uptime: AtomicCell::new(0),
                bytes_complete: AtomicCell::new(0),
                pieces_downloaded: AtomicCell::new(0),
            })
        }
    }
}
