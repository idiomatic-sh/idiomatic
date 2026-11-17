//! The types of the plan's number proof.

/// Whether a column is proven to hold a number on every row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumberProof {
    /// Every row holds a number there.
    Proven,
    /// No proof was found.
    Unproven,
}
