//! The sources port, implemented with `std::fs` and BLAKE3: a path is read only when it lies beneath the root,
//! lexically and after following its links, so a tool cannot make asymptote read outside the product. A file's stamp
//! is the digest of its bytes; its state is its size and modification time, which the core compares with what it
//! recorded to decide which files to read.

use std::path::{Component, Path};
use std::time::{SystemTime, UNIX_EPOCH};

use super::settings::SourcesDiskSettings;
use crate::domain::manifest::types::ProductRoot;
use crate::domain::reports::constants::ABSENT;
use crate::domain::reports::types::{
    Digest, FileDigest, FileDigests, FileStat, FileState, Moment, Size, Stats,
};
use crate::domain::violations::types::{Located, RepoPath, SourceText};
use crate::outbound::sources::errors::SourcesError;
use crate::outbound::sources::interface::SourcesPort;

/// The disk sources adapter.
#[derive(Debug)]
pub(crate) struct SourcesDiskAdapter {
    /// The product it reads.
    pub settings: SourcesDiskSettings,
}

impl SourcesDiskAdapter {
    /// The adapter over the product at `root`.
    pub(crate) const fn new(root: ProductRoot) -> Self {
        Self {
            settings: SourcesDiskSettings { root },
        }
    }
}

/// `time` in nanoseconds since the Unix epoch; none before it.
fn moment(time: SystemTime) -> Option<Moment> {
    time.duration_since(UNIX_EPOCH)
        .ok() // ignore: idiomatic.arch/no-ok-on-result - a time before the epoch is no moment the stat can compare, so the file is read again
        .map(|since| Moment(since.as_nanos()))
}

impl SourcesPort for SourcesDiskAdapter {
    fn read(&self, path: &RepoPath) -> Result<SourceText, SourcesError> {
        let root = &self.settings.root;
        let outside = || SourcesError::Outside {
            path: path.0.to_string(),
        };
        let relative = Path::new(&*path.0);
        if !relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err(outside());
        }
        let unreadable = |error: std::io::Error| SourcesError::Unreadable {
            path: path.0.to_string(),
            reason: error.to_string(),
        };
        let base = Path::new(&*root.0).canonicalize().map_err(unreadable)?;
        let file = base.join(relative).canonicalize().map_err(unreadable)?;
        if !file.starts_with(&base) {
            return Err(outside());
        }
        // A file is decoded lossily, so a byte that is no UTF-8 elsewhere in it keeps a line's key.
        std::fs::read(file)
            .map(|bytes| SourceText(String::from_utf8_lossy(&bytes).into()))
            .map_err(unreadable)
    }

    fn stats(&self, paths: &[RepoPath]) -> Stats {
        let root = &self.settings.root;
        // Taken before any file is looked at: a write after it is modified no earlier than this, less a tick.
        let taken = moment(SystemTime::now()).unwrap_or_default();
        let files = paths
            .iter()
            .map(|path| {
                let state = std::fs::metadata(Path::new(&*root.0).join(&*path.0))
                    .ok() // ignore: idiomatic.arch/no-ok-on-result - a file whose metadata cannot be read is stated absent, so its digest is taken by reading it, which says why it fails
                    .and_then(|metadata| {
                        Some(FileState::Present {
                            size: Size(metadata.len()),
                            modified: moment(metadata.modified().ok()?)?, // ignore: idiomatic.arch/no-ok-on-result - a platform with no modification time states the file absent, so its digest is taken by reading it
                        })
                    })
                    .unwrap_or(FileState::Absent);
                FileStat {
                    path: path.clone(),
                    state,
                }
            })
            .collect();
        Stats { files, taken }
    }

    fn digests(&self, paths: &[RepoPath]) -> FileDigests {
        let root = &self.settings.root;
        FileDigests(
            paths
                .iter()
                .map(|path| FileDigest {
                    path: path.clone(),
                    digest: std::fs::read(Path::new(&*root.0).join(&*path.0)).map_or_else(
                        |_| Digest(ABSENT.into()),
                        |bytes| Digest(blake3::hash(&bytes).to_hex().as_str().into()),
                    ),
                })
                .collect(),
        )
    }

    fn locate(&self, path: &RepoPath) -> Located {
        let root = &self.settings.root;
        let (Ok(base), Ok(file)) = (
            Path::new(&*root.0).canonicalize(),
            Path::new(&*root.0).join(&*path.0).canonicalize(),
        ) else {
            return Located::Inside;
        };
        if file.starts_with(&base) {
            Located::Inside
        } else {
            Located::Outside(RepoPath(file.display().to_string().into()))
        }
    }
}
