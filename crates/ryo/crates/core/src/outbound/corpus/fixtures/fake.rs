//! The fake of the `corpus` port: what the core's tests stand in for it with.

use super::super::errors::CorpusError;
use super::super::interface::CorpusPort;
use crate::outbound::corpus::types::{Inventory, PathStat};
use ryo_values::domain::files::types::{CorpusEntry, CorpusFile, FileBytes, FileSize, Nanoseconds};

/// What the core's tests stand in for `corpus` with: it lists the files it was given, fails when told to, and cannot read a file
/// whose path contains `unstable` twice alike. The files it holds outside the listing are read and stat'ed by their paths alone.
#[derive(Debug, Default)]
pub struct CorpusFake {
    /// The files it lists.
    pub files: Vec<CorpusFile>,
    /// Whether every listing fails.
    pub fails: bool,
    /// How many files it was asked to read, shared with the test that reads it.
    pub reads: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// The files on the disk it does not list, such as those above the root: read and stat'ed by path only.
    pub outside: Vec<CorpusFile>,
}

/// The text in the paths the fake reads as changing while they are read.
pub(crate) const UNSTABLE: &str = "unstable";

/// What the fake answers when it refuses.
fn refusal() -> CorpusError {
    CorpusError::Unreadable {
        path: ".".to_owned(),
        reason: "the fake was told to fail".to_owned(),
    }
}

/// When the fake's file was written: it is dated by what it holds, so an edit changes the date.
fn written(bytes: &[u8]) -> Nanoseconds {
    let mut hasher = rustc_hash::FxHasher::default();
    std::hash::Hasher::write(&mut hasher, bytes);
    Nanoseconds(std::hash::Hasher::finish(&hasher) | 1)
}

impl CorpusPort for CorpusFake {
    fn inventory(&self) -> Result<Inventory, CorpusError> {
        if self.fails {
            return Err(refusal());
        }
        Ok(Inventory(
            self.files
                .iter()
                .map(|file| CorpusEntry {
                    executable: file.executable,
                    path: file.path.clone(),
                    size: FileSize(u64::try_from(file.bytes.0.len()).unwrap_or(u64::MAX)),
                    modified: written(&file.bytes.0),
                })
                .collect(),
        ))
    }

    fn read(&self, entry: &CorpusEntry, into: &mut FileBytes) -> Result<(), CorpusError> {
        let _reads_before = self
            .reads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let into = &mut into.0;
        into.clear();
        if entry.path.0.contains(UNSTABLE) {
            return Err(CorpusError::Unstable {
                path: entry.path.0.to_string(),
            });
        }
        let file = self
            .files
            .iter()
            .chain(&self.outside)
            .find(|file| file.path == entry.path)
            .ok_or_else(refusal)?;
        into.extend_from_slice(&file.bytes.0);
        Ok(())
    }

    fn stat(
        &self,
        path: &ryo_language::domain::files::types::FilePath,
    ) -> Result<PathStat, CorpusError> {
        Ok(self
            .files
            .iter()
            .chain(&self.outside)
            .find(|file| file.path == *path)
            .map(|file| CorpusEntry {
                executable: file.executable,
                path: file.path.clone(),
                size: FileSize(u64::try_from(file.bytes.0.len()).unwrap_or(u64::MAX)),
                modified: written(&file.bytes.0),
            })
            .map_or(PathStat::Missing, PathStat::Found))
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
