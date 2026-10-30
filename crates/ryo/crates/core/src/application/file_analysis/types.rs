//! State owned by a bounded file analysis invocation.
use super::errors::FileAnalysisError;
use crate::domain::workers::types::{Tables, VisitSetting};
use crate::outbound::facts::interface::FactsWorker;
/// What one worker owns while the jobs run: its tables, its handle of the fact store and the first thing that went wrong in it.
pub(crate) struct Worker {
    /// What the worker owns besides its handle.
    pub tables: Tables,
    /// Where the rows of the file being read and the words kept of it are.
    pub facts: Box<dyn FactsWorker>,
    /// The failure that stopped the worker, when one did.
    pub failure: Option<FileAnalysisError>,
}

/// What a visit of a file reads that is the same for every file, and the ports a visit reaches besides.
pub(crate) struct FileVisit<'a> {
    /// What the domain reads.
    pub setting: VisitSetting<'a>,
}

use crate::domain::visit::types::{Noting, RoleRead};
use crate::outbound::language::types::{NoteWriting, RoleWriting};
impl From<Noting> for NoteWriting {
    fn from(noting: Noting) -> Self {
        match noting {
            Noting::Wanted => Self::Wanted,
            Noting::Unwanted => Self::Unwanted,
        }
    }
}
impl From<RoleRead> for RoleWriting {
    fn from(read: RoleRead) -> Self {
        match read {
            RoleRead::Reads => Self::Wanted,
            RoleRead::Skips => Self::Unwanted,
        }
    }
}
