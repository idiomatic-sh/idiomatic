//! The names of the `fusing` concept.

use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The first column of a reach's rows, from which its columns are numbered: its partition keys, then its source and destination.
pub(crate) const FIRST_REACH_COLUMN: ColumnIndex = ColumnIndex(0);
