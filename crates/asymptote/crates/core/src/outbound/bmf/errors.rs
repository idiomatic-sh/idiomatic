//! The `bmf` port's failures.

/// Why a tool's output could not be read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BmfError {
    /// It is not Bencher Metric Format: no output, or no measurement at all, is not one either.
    #[error("the output is not Bencher Metric Format: {reason}")]
    Unparsable {
        /// What the parser said.
        reason: String,
    },
}
