//! The names of the `narrowing` concept.

use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The one column of the values the key of a join's other input can be, which the narrowed rows are met with.
pub(crate) const VALUE: ColumnIndex = ColumnIndex(0);
