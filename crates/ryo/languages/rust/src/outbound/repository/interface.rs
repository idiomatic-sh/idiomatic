//! What the `repository` port asks of the outside: the files of the repository a language reads besides the ones it is given. Every
//! path is relative to the root of the repository the adapter was built for.

use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::FileText;
use crate::domain::paths::types::Height;
use crate::domain::paths::types::Placement;
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::WrittenPath;
use crate::outbound::repository::types::{Entries, Looks};

/// What every `repository` adapter implements.
pub(crate) trait RepositoryPort: Send + Sync {
    /// The text of the file at `path`; none when it cannot be read.
    fn text(&self, path: &RepoPath) -> Section<FileText>;

    /// What is in the directory `directory`; nothing when it cannot be read.
    fn entries(&self, directory: &RepoPath) -> Entries;

    /// The files called `name` in `directory` and in each directory above it up to the top of the file system, nearest first; each is
    /// looked at whether it is there or not.
    fn above(&self, directory: &RepoPath, name: &EntryName) -> Looks;

    /// Where `written`, a path a file of the directory `height` above `directory` writes, lands: at a path from the root, or outside
    /// the repository.
    fn placed(&self, directory: &RepoPath, height: Height, written: &WrittenPath) -> Placement;
}
