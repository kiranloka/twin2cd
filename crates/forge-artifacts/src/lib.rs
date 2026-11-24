//! Local artifact and cache storage. Names are opaque keys, never paths.

use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use thiserror::Error;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("invalid artifact name: {0}")]
    InvalidName(String),
    #[error("artifact '{0}' was not found")]
    NotFound(String),
    #[error("artifact I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct ArtifactRegistry {
    root: PathBuf,
}

impl Default for ArtifactRegistry {
    fn default() -> Self {
        let root = std::env::var_os("FORGE_ARTIFACTS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".forge/artifacts"));
        Self::new(root)
    }
}

impl ArtifactRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub async fn put(
        &self,
        name: &str,
        source: impl AsRef<Path>,
    ) -> Result<ArtifactRecord, ArtifactError> {
        self.copy_in("artifacts", name, source.as_ref()).await
    }

    pub async fn get(
        &self,
        name: &str,
        destination: impl AsRef<Path>,
    ) -> Result<u64, ArtifactError> {
        self.copy_out("artifacts", name, destination.as_ref()).await
    }

    pub async fn cache_store(
        &self,
        key: &str,
        source: impl AsRef<Path>,
    ) -> Result<ArtifactRecord, ArtifactError> {
        self.copy_in("cache", key, source.as_ref()).await
    }

    pub async fn cache_restore(
        &self,
        key: &str,
        destination: impl AsRef<Path>,
    ) -> Result<u64, ArtifactError> {
        self.copy_out("cache", key, destination.as_ref()).await
    }

    async fn copy_in(
        &self,
        namespace: &str,
        name: &str,
        source: &Path,
    ) -> Result<ArtifactRecord, ArtifactError> {
        validate_name(name)?;
        let source_metadata = fs::symlink_metadata(source).await?;
        if !source_metadata.file_type().is_file() {
            return Err(ArtifactError::InvalidName(
                "source must be a regular file".into(),
            ));
        }
        let directory = self.root.join(namespace);
        fs::create_dir_all(&directory).await?;
        let target = directory.join(name);
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = directory.join(format!(".tmp-{}-{sequence}", std::process::id()));
        let copy_result = async {
            let mut input = fs::File::open(source).await?;
            let mut output = fs::File::create(&temporary).await?;
            let mut hasher = Sha256::new();
            let mut size_bytes = 0u64;
            let mut buffer = vec![0u8; 64 * 1024];
            loop {
                let read = input.read(&mut buffer).await?;
                if read == 0 {
                    break;
                }
                output.write_all(&buffer[..read]).await?;
                hasher.update(&buffer[..read]);
                size_bytes += read as u64;
            }
            output.flush().await?;
            output.sync_all().await?;
            Ok::<_, std::io::Error>((size_bytes, hex::encode(hasher.finalize())))
        }
        .await;
        let (size_bytes, sha256) = match copy_result {
            Ok(result) => result,
            Err(error) => {
                let _ = fs::remove_file(&temporary).await;
                return Err(error.into());
            }
        };
        if let Err(error) = fs::rename(&temporary, &target).await {
            let _ = fs::remove_file(&temporary).await;
            return Err(error.into());
        }
        Ok(ArtifactRecord {
            name: name.to_owned(),
            path: target,
            size_bytes,
            sha256,
        })
    }

    async fn copy_out(
        &self,
        namespace: &str,
        name: &str,
        destination: &Path,
    ) -> Result<u64, ArtifactError> {
        validate_name(name)?;
        let source = self.root.join(namespace).join(name);
        let metadata = fs::symlink_metadata(&source)
            .await
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => ArtifactError::NotFound(name.to_owned()),
                _ => ArtifactError::Io(error),
            })?;
        if !metadata.file_type().is_file() {
            return Err(ArtifactError::InvalidName(
                "stored artifact is not a regular file".into(),
            ));
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::copy(source, destination).await?;
        Ok(metadata.len())
    }
}

fn validate_name(name: &str) -> Result<(), ArtifactError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    {
        return Err(ArtifactError::InvalidName(name.to_owned()));
    }
    Ok(())
}
