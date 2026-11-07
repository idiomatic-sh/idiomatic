//! What the `corpus` port asks of the outside: the files of a repository.

use super::errors::CorpusError;
use crate::outbound::corpus::types::{CorpusInventory, PathStat};
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::project_context::types::EntryObservation;
use ryo_values::domain::files::types::{CorpusEntry, FileBytes};

/// What every `corpus` adapter implements.
pub trait CorpusPort: Send + Sync {
    /// Finds the files of the repository the adapter was made over, with their sizes, in no order. Files the repository ignores
    /// and the `.git` directory are not listed. No file is read.
    ///
    /// # Errors
    ///
    /// `Unreadable` when the root or something below it cannot be walked.
    fn inventory(&self) -> Result<CorpusInventory, CorpusError>;

    /// Reads the file `entry` names into `into`, replacing what it held and making room for its size first: the caller keeps one
    /// buffer for all its files.
    ///
    /// # Errors
    ///
    /// `Unreadable` when the file cannot be read.
    fn read(&self, entry: &CorpusEntry, into: &mut FileBytes) -> Result<(), CorpusError>;

    /// What the system says of the file at `path`, relative to the root, listed or not (`../` climbs above the root): its size and
    /// when it was last written; none when it is not there or is not a file. Nothing is read.
    ///
    /// # Errors
    ///
    /// `Unreadable` when the system cannot say what the path is.
    fn stat(&self, path: &FilePath) -> Result<PathStat, CorpusError>;

    /// Observe a candidate entry's kind without reading bytes; directory, absence and uncertainty stay distinct.
    fn observe_entry(&self, path: &FilePath) -> EntryObservation;
}
