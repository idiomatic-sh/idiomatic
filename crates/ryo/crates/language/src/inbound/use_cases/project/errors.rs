//! The `project` use case's failures.

use crate::domain::reasons::types::ToolMessage;

/// Why `project` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProjectError {
    /// A manifest does not parse.
    #[error("a manifest does not parse: {reason}")]
    Manifest {
        /// What was wrong.
        reason: ToolMessage,
    },
}
