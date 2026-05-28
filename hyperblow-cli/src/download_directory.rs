use std::{
    env, fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

const DEFAULT_DOWNLOAD_DIRECTORY_NAME: &str = "hyperblow_downloads";

#[derive(Debug, Error)]
pub enum DownloadDirectoryError {
    #[error("HOME is not set, cannot resolve default download directory")]
    MissingHome,

    #[error("download directory path exists but is not a directory: {0}")]
    NotDirectory(String),

    #[error("could not create download directory: {path}")]
    Create { path: String, source: io::Error },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadDirectory {
    path: PathBuf,
}

impl DownloadDirectory {
    pub fn default() -> Result<Self, DownloadDirectoryError> {
        let home = env::var_os("HOME").ok_or(DownloadDirectoryError::MissingHome)?;
        Ok(Self::from_path(PathBuf::from(home).join(DEFAULT_DOWNLOAD_DIRECTORY_NAME)))
    }

    pub fn from_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn ensure_exists(&self) -> Result<(), DownloadDirectoryError> {
        if self.path.exists() && !self.path.is_dir() {
            return Err(DownloadDirectoryError::NotDirectory(self.path.display().to_string()));
        }

        fs::create_dir_all(&self.path).map_err(|source| DownloadDirectoryError::Create {
            path: self.path.display().to_string(),
            source,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn display_path(&self) -> String {
        self.path.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{DownloadDirectory, DEFAULT_DOWNLOAD_DIRECTORY_NAME};
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn default_directory_is_under_home() {
        let Some(home) = env::var_os("HOME") else {
            return;
        };

        let directory = DownloadDirectory::default().expect("HOME should resolve default directory");

        assert_eq!(directory.path(), PathBuf::from(home).join(DEFAULT_DOWNLOAD_DIRECTORY_NAME));
    }

    #[test]
    fn ensure_exists_creates_directory() {
        let path = DownloadDirectoryTestHarness::temp_path();
        let directory = DownloadDirectory::from_path(path.clone());

        directory.ensure_exists().expect("directory should be created");

        assert!(path.is_dir());
        fs::remove_dir_all(path).expect("temp directory should be removed");
    }

    struct DownloadDirectoryTestHarness;

    impl DownloadDirectoryTestHarness {
        fn temp_path() -> PathBuf {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos();
            PathBuf::from("target")
                .join("download-directory-tests")
                .join(format!("t{}{}", std::process::id(), nonce % 1_000_000_000))
        }
    }
}
