//! The `manifest` port's failures.
use crate::domain::workspaces::types::Reason;

/// Why an operation of `manifest` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ManifestError {
    /// The text is not what it should be.
    #[error("{reason}")]
    Unreadable {
        /// What the reader said.
        reason: Reason,
    },
}
