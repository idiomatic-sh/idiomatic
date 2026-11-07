//! The failures of `operator describe`.

/// Why `describe` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DescribeError {
    /// No operator has that name.
    #[error("no operator named `{name}`")]
    UnknownOperator {
        /// The name asked for.
        name: String,
    },
}
