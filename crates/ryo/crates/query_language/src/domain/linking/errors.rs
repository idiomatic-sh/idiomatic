//! Refusals raised while linking a parsed query bundle.

use ryo_values::domain::program::types::QueryOrigin;

/// A module-linking error located in its original source and import/call trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkError {
    /// The source, byte span and expansion trace that caused the refusal.
    pub origin: QueryOrigin,
    /// Why linking failed.
    pub message: String,
}
