//! The fake of the `sources` port: files held in memory.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::domain::reports::types::{
    Digest, FileDigest, FileDigests, FileStat, FileState, Moment, Size, Stats,
};
use crate::domain::violations::types::{Located, RepoPath, SourceText};
use crate::outbound::sources::errors::SourcesError;
use crate::outbound::sources::interface::SourcesPort;

/// What the core's tests stand in for `sources` with.
#[derive(Debug, Default)]
pub struct SourcesFake {
    /// The files, by path.
    pub files: Mutex<BTreeMap<RepoPath, SourceText>>,
    /// The paths that lead outside the product once their links are followed, with where.
    pub outside: BTreeMap<RepoPath, RepoPath>,
    /// How many times each file was written since the fake was made: its modification time.
    // ignore: idiomatic.arch/pub-item-unreferenced-elsewhere - the fake's own record of each write, which `write` keeps and `stamps` reads as a modification time; the fake keeps one visibility for every field (clippy::partial_pub_fields)
    pub writes: Mutex<BTreeMap<RepoPath, u128>>,
    /// How many file contents were read for their digests.
    pub reads: AtomicUsize,
}

impl SourcesFake {
    /// The fake holding `files`, each `(path, text)`.
    #[must_use]
    pub fn holding(files: &[(&str, &str)]) -> Self {
        Self {
            files: Mutex::new(
                files
                    .iter()
                    .map(|(path, text)| (RepoPath((*path).into()), SourceText((*text).into())))
                    .collect(),
            ),
            ..Self::default()
        }
    }

    /// How many file contents the stamps read so far.
    #[must_use]
    pub fn contents_read(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }

    /// Writes `text` at `path`.
    pub fn write(&self, path: &str, text: &str) {
        // A write replaces whatever text the path held, as a file's would.
        self.files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend([(RepoPath(path.into()), SourceText(text.into()))]);
        *self
            .writes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(RepoPath(path.into()))
            .or_default() += 1;
    }

    /// The state `path` has now: its size, and its writes as its modification time; absent when it is not held.
    fn state(&self, path: &RepoPath) -> FileState {
        let Some(text) = self.get(path) else {
            return FileState::Absent;
        };
        let written = self
            .writes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .copied()
            .unwrap_or_default();
        FileState::Present {
            size: Size(u64::try_from(text.0.len()).unwrap_or(u64::MAX)),
            modified: Moment(written),
        }
    }
    /// The text written at `path`, when one is.
    fn get(&self, path: &RepoPath) -> Option<SourceText> {
        self.files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .cloned()
    }
}

impl SourcesPort for SourcesFake {
    fn read(&self, path: &RepoPath) -> Result<SourceText, SourcesError> {
        if path.0.starts_with('/') || path.0.split('/').any(|segment| segment == "..") {
            return Err(SourcesError::Outside {
                path: path.0.to_string(),
            });
        }
        self.get(path).ok_or_else(|| SourcesError::Unreadable {
            path: path.0.to_string(),
            reason: "the fake holds no such file".to_owned(),
        })
    }

    fn stats(&self, paths: &[RepoPath]) -> Stats {
        Stats {
            files: paths
                .iter()
                .map(|path| FileStat {
                    path: path.clone(),
                    state: self.state(path),
                })
                .collect(),
            // Long after any write the fake counts, so no record it vouches for is racy.
            taken: Moment(u128::from(u64::MAX)),
        }
    }

    fn digests(&self, paths: &[RepoPath]) -> FileDigests {
        FileDigests(
            paths
                .iter()
                .map(|path| FileDigest {
                    path: path.clone(),
                    digest: self.get(path).map_or_else(
                        || Digest("absent".into()),
                        |text| {
                            let _read_before = self.reads.fetch_add(1, Ordering::Relaxed);
                            Digest(text.0)
                        },
                    ),
                })
                .collect(),
        )
    }

    fn locate(&self, path: &RepoPath) -> Located {
        self.outside
            .get(path)
            .map_or(Located::Inside, |target| Located::Outside(target.clone()))
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
