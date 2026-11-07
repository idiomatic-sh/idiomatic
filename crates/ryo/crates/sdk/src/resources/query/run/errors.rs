//! The failures of `query run`.

use std::path::PathBuf;

/// Why `run` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RunError {
    /// The root is not text, which the engine reads paths as.
    #[error("the root {} is not text", path.display())]
    PathNotText {
        /// The root.
        path: PathBuf,
    },
    /// The query text is wrong.
    #[error("{reason}")]
    Query {
        /// Where and how.
        reason: String,
    },
    /// The repository could not be listed.
    #[error("corpus: {reason}")]
    Corpus {
        /// What the corpus port said.
        reason: String,
    },
    /// The fact store failed.
    #[error("facts: {reason}")]
    Facts {
        /// What the facts port said.
        reason: String,
    },
    /// The relation catalog could not be read.
    #[error("language: {reason}")]
    Language {
        /// What the language port said.
        reason: String,
    },
    /// Memory could not be read.
    #[error("memory: {reason}")]
    Memory {
        /// Why.
        reason: String,
    },
    /// The repository does not fit the memory budget.
    #[error("the repository is {size} bytes and the budget is {budget}")]
    TooLarge {
        /// The bytes of the repository.
        size: u64,
        /// The bytes the run may use.
        budget: u64,
    },
    /// The workers could not be started.
    #[error("scheduler: {reason}")]
    Scheduler {
        /// What the scheduler port said.
        reason: String,
    },
    /// An operator refused its input.
    #[error("{reason}")]
    Evaluate {
        /// What the operator said.
        reason: String,
    },
    /// The answer could not be written.
    #[error("output: {reason}")]
    Output {
        /// What the rows port said.
        reason: String,
    },
    /// Whoever reads the answer stopped reading, so it was not finished.
    #[error("whoever reads the answer stopped reading")]
    OutputClosed,
    /// The process held more memory than the budget while it read the repository: refused before the machine runs out.
    #[error("the run holds {used} bytes and the budget is {budget}")]
    MemoryBudgetExceeded {
        /// The bytes the process held.
        used: u64,
        /// The bytes the run may use.
        budget: u64,
    },
}
