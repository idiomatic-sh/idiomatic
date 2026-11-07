//! The `project` use case's failures.

use crate::domain::reasons::types::ToolMessage;

/// Why `project` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProjectError {
    /// Required repository context could not be established.
    #[error("project context: {reason}")]
    Context {
        /// The lookup failure, retaining the path and outside cause.
        reason: ToolMessage,
    },
    /// A manifest does not parse.
    #[error("a manifest does not parse: {reason}")]
    Manifest {
        /// What was wrong.
        reason: ToolMessage,
    },
}
