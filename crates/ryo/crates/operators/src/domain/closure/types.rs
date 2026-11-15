//! What a closure reads from the engine: its partitions, their sources and the walk over them.

/// The graph a closure walks: the partition columns of an edge row, its source column and its destination column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReachParts {
    /// The columns that make a partition, in output order; none means one graph.
    pub(crate) keys: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The source column of an edge row.
    pub(crate) source: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The destination column of an edge row.
    pub(crate) destination: ryo_values::domain::plan_parts::types::ColumnIndex,
}
