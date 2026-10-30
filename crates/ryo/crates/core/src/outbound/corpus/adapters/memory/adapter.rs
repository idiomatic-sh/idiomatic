//! The corpus port, implemented with the files the caller gave as text: the files of a case, read instead of the disk. The root names
//! nothing.

use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::{Inventory, PathStat};
use ryo_values::domain::files::types::{CorpusEntry, CorpusFile, FileBytes, FileSize, Nanoseconds};

/// The memory corpus adapter: the files it was given.
#[derive(Debug)]
pub(crate) struct CorpusMemoryAdapter {
    /// The files it lists and reads.
    files: Vec<CorpusFile>,
}

impl CorpusMemoryAdapter {
    /// An adapter over `files`.
    pub(crate) fn new(files: Vec<CorpusFile>) -> Self {
        Self { files }
    }
}

/// When a file given as text was written: it is dated by what it holds, so an edit changes the date.
fn written(bytes: &FileBytes) -> Nanoseconds {
    let mut hasher = rustc_hash::FxHasher::default();
    std::hash::Hasher::write(&mut hasher, &bytes.0);
    Nanoseconds(std::hash::Hasher::finish(&hasher) | 1)
}

impl CorpusPort for CorpusMemoryAdapter {
    fn inventory(&self) -> Result<Inventory, CorpusError> {
        Ok(Inventory(
            self.files
                .iter()
                .map(|file| CorpusEntry {
                    path: file.path.clone(),
                    size: FileSize(u64::try_from(file.bytes.0.len()).unwrap_or(u64::MAX)),
                    modified: written(&file.bytes),
                })
                .collect(),
        ))
    }

    fn read(&self, entry: &CorpusEntry, into: &mut FileBytes) -> Result<(), CorpusError> {
        let file = self
            .files
            .iter()
            .find(|file| file.path == entry.path)
            .ok_or_else(|| CorpusError::Unreadable {
                path: entry.path.0.to_string(),
                reason: super::constants::NOT_GIVEN.to_owned(),
            })?;
        into.0.clear();
        into.0.extend_from_slice(&file.bytes.0);
        Ok(())
    }

    fn stat(
        &self,
        path: &ryo_language::domain::files::types::FilePath,
    ) -> Result<PathStat, CorpusError> {
        Ok(self
            .files
            .iter()
            .find(|file| file.path == *path)
            .map(|file| CorpusEntry {
                path: file.path.clone(),
                size: FileSize(u64::try_from(file.bytes.0.len()).unwrap_or(u64::MAX)),
                modified: written(&file.bytes),
            })
            .map_or(PathStat::Missing, PathStat::Found))
    }
}
