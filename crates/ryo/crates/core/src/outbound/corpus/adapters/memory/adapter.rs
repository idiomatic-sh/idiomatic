//! The corpus port, implemented with the files the caller gave as text: the files of a case, read instead of the disk. The root names
//! nothing.

use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::{CorpusInventory, PathStat};
use ryo_language::domain::project_context::types::{
    EntryObservation, ObservedEntryState, GivenProjectInventory, ProjectInventoryCompleteness,
};
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusFile, CorpusInput, FileBytes, FileSize, Nanoseconds,
};
use std::collections::{BTreeMap, BTreeSet};

/// The memory corpus adapter: the files it was given.
#[derive(Debug)]
pub(crate) struct CorpusMemoryAdapter {
    /// The files it lists and reads.
    files: Vec<CorpusFile>,
    /// Explicit host inventory of contextual entries outside the source texts.
    project_entries: BTreeMap<ryo_language::domain::files::types::FilePath, ObservedEntryState>,
    project_completeness: ProjectInventoryCompleteness,
    source_paths: BTreeSet<ryo_language::domain::files::types::FilePath>,
    /// Parent directories proved by supplied source files or known present entries.
    parent_directories: BTreeSet<ryo_language::domain::files::types::FilePath>,
}

impl CorpusMemoryAdapter {
    /// An adapter over `files`.
    pub(crate) fn new(files: Vec<CorpusFile>, project_inventory: GivenProjectInventory) -> Self {
        let mut project_entries = BTreeMap::new();
        let mut parent_directories = BTreeSet::new();
        for entry in project_inventory.entries {
            if matches!(
                entry.state,
                ObservedEntryState::File | ObservedEntryState::Directory | ObservedEntryState::Other
            ) {
                parents_of(&entry.path, &mut parent_directories);
            }
            match project_entries.entry(entry.path) {
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(entry.state);
                }
                std::collections::btree_map::Entry::Occupied(mut slot) => {
                    slot.insert(ObservedEntryState::Unknown);
                }
            }
        }
        let source_paths = files.iter().map(|file| file.path.clone()).collect();
        for file in &files {
            parents_of(&file.path, &mut parent_directories);
        }
        Self {
            files,
            project_entries,
            project_completeness: project_inventory.completeness,
            source_paths,
            parent_directories,
        }
    }
}

/// A known child proves each of its repository-relative parent directories exists.
fn parents_of(
    path: &ryo_language::domain::files::types::FilePath,
    into: &mut BTreeSet<ryo_language::domain::files::types::FilePath>,
) {
    let mut parent = path.0.rsplit_once('/').map(|(parent, _)| parent);
    while let Some(path) = parent {
        if path.is_empty() {
            break;
        }
        into.insert(ryo_language::domain::files::types::FilePath(path.into()));
        parent = path.rsplit_once('/').map(|(next, _)| next);
    }
}

/// When a file given as text was written: it is dated by what it holds, so an edit changes the date.
fn written(bytes: &FileBytes) -> Nanoseconds {
    let mut hasher = rustc_hash::FxHasher::default();
    std::hash::Hasher::write(&mut hasher, &bytes.0);
    Nanoseconds(std::hash::Hasher::finish(&hasher) | 1)
}

impl CorpusPort for CorpusMemoryAdapter {
    fn inventory(&self) -> Result<CorpusInventory, CorpusError> {
        Ok(CorpusInventory(
            self.files
                .iter()
                .map(|file| CorpusEntry {
                    executable: file.executable,
                    path: file.path.clone(),
                    size: FileSize(u64::try_from(file.bytes.0.len()).unwrap_or(u64::MAX)),
                    modified: written(&file.bytes),
                })
                .map(CorpusInput::Available)
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
                executable: file.executable,
                path: file.path.clone(),
                size: FileSize(u64::try_from(file.bytes.0.len()).unwrap_or(u64::MAX)),
                modified: written(&file.bytes),
            })
            .map_or(PathStat::Missing, PathStat::Found))
    }

    fn observe_entry(
        &self,
        path: &ryo_language::domain::files::types::FilePath,
    ) -> EntryObservation {
        let supplied = self.project_entries.get(path).copied();
        let source_file = self.source_paths.contains(path);
        let parent_directory = self.parent_directories.contains(path);
        let state = if supplied == Some(ObservedEntryState::Unknown) || (source_file && parent_directory) {
            ObservedEntryState::Unknown
        } else if source_file {
            if supplied.is_none_or(|state| state == ObservedEntryState::File) {
                ObservedEntryState::File
            } else {
                ObservedEntryState::Unknown
            }
        } else if parent_directory {
            if supplied.is_none_or(|state| state == ObservedEntryState::Directory) {
                ObservedEntryState::Directory
            } else {
                ObservedEntryState::Unknown
            }
        } else if let Some(state) = supplied {
            state
        } else if self.project_completeness
            == ProjectInventoryCompleteness::CompleteWithinDeclaredScope
        {
            ObservedEntryState::Missing
        } else {
            ObservedEntryState::Unknown
        };
        EntryObservation {
            path: path.clone(),
            state,
        }
    }
}
