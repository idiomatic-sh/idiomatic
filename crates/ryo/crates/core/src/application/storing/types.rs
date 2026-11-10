//! The types of the `storing` operation: the repository as the run lists it, and what it decided of what the last run kept.
use crate::domain::storing::types::Decision;
use crate::outbound::lint_store::interface::ReadStore;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::runs::types::MemoryBytes;
use ryo_values::domain::stores::types::Stored;
use std::sync::Arc;

/// The repository as this run finds it, and what the last run kept of it.
pub(crate) struct RepositoryListing {
    /// The files, in the order of their paths.
    pub files: Vec<CorpusInput>,
    /// What the last run of this build kept; empty when it kept nothing.
    pub stored: Stored,
    /// The store those tables were read from, none when the run reads none: what this run reads of it later, and keeps against, is of
    /// it alone.
    pub read: Option<Arc<dyn ReadStore>>,
    /// The most memory the run may hold.
    pub budget: MemoryBytes,
}

/// What one of the two jobs that open a lint answered: the files under the root, or what the last run kept, each or the
/// error `E` it failed with.
#[derive(Debug)]
pub(crate) enum Listed<E> {
    /// The files under the root.
    Files(Result<Vec<CorpusInput>, E>),
    /// What the last run of this build kept, boxed: it is far larger than a list of files.
    Stored(Result<Option<Arc<dyn ReadStore>>, E>),
}

/// What the run decided of what the last kept, before the changed files are read alone: the decision, and the changed files.
pub(crate) struct Decided {
    /// What the run does with what the last kept.
    pub decision: Decision,
    /// The changed files the decision reads alone, in the order of their paths.
    pub changed: Vec<CorpusInput>,
}
