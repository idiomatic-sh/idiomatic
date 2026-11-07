//! The `admission` concept's failures.

/// Why the output of a language for a file is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum AdmissionError {
    /// A span ends before it starts, or after the file ends.
    #[error("a span is not within its file")]
    SpanOutsideFile,
    /// A node is at a place the file's table does not have.
    #[error("a node is not in its file's table")]
    NodeOutsideTable,
    /// A node belongs to another file.
    #[error("a node is of another file")]
    NodeOfAnotherFile,
    /// A cell that names a file names one that is not the file visited.
    #[error("a cell names a file that is not the one visited")]
    FileNotVisited,
}
