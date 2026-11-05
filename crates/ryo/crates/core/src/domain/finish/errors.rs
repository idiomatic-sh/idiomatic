//! The `finish` concept's failures.

use ryo_values::domain::rows::errors::TableModelError;

/// Why the rows of a query cannot be read as findings.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum FinishError {
    /// A named publication cannot retain a truthful portable name mapping.
    #[error("{reason}")]
    Publication { reason: Box<str> },
    /// The query's rows lack a column the `emit` names.
    #[error(transparent)]
    Table(#[from] TableModelError),
}
