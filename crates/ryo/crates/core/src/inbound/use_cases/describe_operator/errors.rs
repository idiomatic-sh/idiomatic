//! The `describe_operator` use case's failures.

/// Why `describe_operator` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DescribeOperatorError {
    /// No operator has that name.
    #[error("no operator named `{name}`")]
    UnknownOperator {
        /// The name asked for.
        name: String,
    },
}
