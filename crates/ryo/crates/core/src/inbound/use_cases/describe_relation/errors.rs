//! The `describe_relation` use case's failures.

/// Why `describe_relation` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DescribeRelationError {
    /// No relation has that name.
    #[error("no relation named `{name}`")]
    UnknownRelation {
        /// The name asked for.
        name: String,
    },
    /// The relation catalog could not be read.
    #[error("the relation catalog failed: {reason}")]
    Language {
        /// What the language port said.
        reason: String,
    },
}
