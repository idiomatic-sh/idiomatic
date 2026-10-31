//! The repository port, implemented with the files of the machine: paths are relative to the root the adapter was built for. The
//! file system's own paths stop here: what leaves is a path from the root, or that a place lies outside it.

use crate::outbound::repository::types::{Entries, Looks};
use std::path::{Path, PathBuf};

use super::schemas::{entry_of, text_of_file};
use crate::domain::paths::constants::SEPARATOR;
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::FileText;
use crate::domain::paths::types::Height;
use crate::domain::paths::types::Look;
use crate::domain::paths::types::Placement;
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::WrittenPath;
use crate::outbound::repository::interface::RepositoryPort;
use ryo::languages::RootPath;

/// The `native` repository adapter.
#[derive(Debug)]
pub(crate) struct RepositoryNativeAdapter {
    /// The root of the repository, as the run names it.
    root: RootPath,
}

impl RepositoryNativeAdapter {
    /// The adapter over the files below `root`.
    #[must_use]
    pub(crate) const fn new(root: RootPath) -> Self {
        Self { root }
    }

    /// `path` placed under the root, where `path` is relative and may be empty.
    fn placed_under(&self, path: &RepoPath) -> PathBuf {
        Path::new(&*self.root.0).join(&*path.0)
    }

    /// The root as the file system names it from the top, or as the run names it when it cannot say.
    fn absolute(&self) -> PathBuf {
        std::fs::canonicalize(&*self.root.0).unwrap_or_else(|_| PathBuf::from(&*self.root.0))
    }

    /// `directory` as the file system names it from the top, or placed under the root when it cannot say.
    fn start(&self, directory: &RepoPath) -> PathBuf {
        std::fs::canonicalize(self.placed_under(directory))
            .unwrap_or_else(|_| self.placed_under(directory))
    }
}

/// The text of the file at `path`, when it is there and is text.
fn read(path: &Path) -> Section<FileText> {
    // The port answers a file it cannot read, and one that is not text, as absent, as its contract says.
    match std::fs::read(path) {
        Ok(bytes) => text_of_file(bytes).unwrap_or(Section::Absent),
        Err(_) => Section::Absent,
    }
}

impl RepositoryPort for RepositoryNativeAdapter {
    fn text(&self, path: &RepoPath) -> Section<FileText> {
        read(&self.placed_under(path))
    }

    fn entries(&self, directory: &RepoPath) -> Entries {
        let Ok(listing) = std::fs::read_dir(self.placed_under(directory)) else {
            return Entries(Vec::new());
        };
        Entries(
            listing
                // An entry the system cannot list, or whose name is not text, is not in the listing, as the port's contract says.
                .filter_map(|listed| {
                    let Ok(entry) = listed else {
                        return None;
                    };
                    let Ok(each) = entry_of(entry) else {
                        return None;
                    };
                    Some(each)
                })
                .collect(),
        )
    }

    fn above(&self, directory: &RepoPath, name: &EntryName) -> Looks {
        Looks(
            self.start(directory)
                .ancestors()
                .zip(0_u32..)
                .map(|(each, height)| Look {
                    height: Height(height),
                    text: read(&each.join(&*name.0)),
                })
                .collect(),
        )
    }

    fn placed(&self, directory: &RepoPath, height: Height, written: &WrittenPath) -> Placement {
        let start = self.start(directory);
        let Some(base) = start
            .ancestors()
            .nth(usize::try_from(height.0).unwrap_or(usize::MAX))
        else {
            return Placement::Outside;
        };
        base.join(&*written.0)
            .strip_prefix(self.absolute())
            .map_or(Placement::Outside, |inside| {
                // Joined by `/` on every platform: a repository path is the same record everywhere.
                Placement::Inside(RepoPath(
                    inside
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join(SEPARATOR)
                        .into(),
                ))
            })
    }
}
