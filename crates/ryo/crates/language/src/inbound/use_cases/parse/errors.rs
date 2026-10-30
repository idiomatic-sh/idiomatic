//! The `parse` use case's failures.

use crate::domain::reasons::types::ToolMessage;

/// Why `parse` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// The language does not read this file; another language may.
    #[error("the language does not read this file")]
    Unsupported,
    /// The parser library failed.
    #[error("the parser failed: {reason}")]
    Library {
        /// What the library said.
        reason: ToolMessage,
    },
}
