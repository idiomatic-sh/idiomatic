//! Refusals raised while linking a parsed query bundle.

use ryo_values::domain::program::types::QueryOrigin;

/// Why a parsed query bundle does not link.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum LinkingError {
    /// A module-linking error located in its original source and import/call trace.
    #[error("{message}")]
    Located {
        /// The source, byte span and expansion trace that caused the refusal.
        origin: QueryOrigin,
        /// Why linking failed.
        message: String,
    },
}
