//! Invalid names in bound observation declarations.

use ryo_values::domain::observations::types::{ObservationBindingName, ObservationColumnName};

/// Why an input cannot join the query source namespace.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ObservationInputsError {
    /// An input has no usable name.
    #[error("an observation input name is blank")]
    BlankName,
    /// An input repeats or replaces a visible source.
    #[error("observation input `{}` repeats or shadows an existing source", name.0)]
    SourceCollision {
        /// The conflicting input name.
        name: ObservationBindingName,
    },
    /// Columns must have distinct nonblank names within their input.
    #[error("observation input `{}` has a blank or repeated column `{}`", input.0, column.0)]
    Column {
        /// The input owning the invalid column.
        input: ObservationBindingName,
        /// The blank or repeated column.
        column: ObservationColumnName,
    },
}
