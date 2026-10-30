//! The `finish` concept's failures.

use super::super::table_model::errors::TableModelError;

/// Why the rows of a query cannot be read as findings.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum FinishError {
    /// The query's rows lack a column the `emit` names.
    #[error(transparent)]
    Table(#[from] TableModelError),
}
