//! The names of the `table_model` concept: how a typed value is written when it leaves the engine.

/// The heading of the column that says on which line a row is.
#[must_use]
pub(crate) fn line_heading() -> &'static str {
    "line"
}

/// The heading of the column that says at which character of its line a row is.
#[must_use]
pub(crate) fn column_heading() -> &'static str {
    "column"
}
