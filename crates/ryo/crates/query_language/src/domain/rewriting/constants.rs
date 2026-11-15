//! The names of the `rewriting` concept.

use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The column of a pair's rows that holds its first function.
pub(crate) const PAIR_FUNCTION: ColumnIndex = ColumnIndex(0);

/// The column of a pair's rows that holds its other function.
pub(crate) const PAIR_OTHER: ColumnIndex = ColumnIndex(1);

/// The column of a pair joined with the texts of one of its functions that holds the text, after the pair's two functions.
pub(crate) const JOINED_TEXT: ColumnIndex = ColumnIndex(2);
