//! The `rows` concept's failures.

/// Why an operation on a table failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RowsError {
    /// A step reads a column its table does not have.
    #[error("the table has no column {index}")]
    NoSuchColumn {
        /// The position asked for.
        index: u32,
    },
}
