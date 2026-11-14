//! The names of the `closure_adjacency` concept.

use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The column of a partition's table that holds an edge's source, before its destination: the column the partition is indexed on.
pub(crate) const SOURCE: ColumnIndex = ColumnIndex(0);
