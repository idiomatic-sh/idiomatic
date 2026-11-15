//! One row for each vertex a walk reaches from each source, partition by partition, in walk order.

/// The graph of a closure over edges whose `keys` make a partition and whose `source` leads to `destination`: its one
/// lowering, walked from every source or from the sources its join binds.
pub(crate) fn lower(
    keys: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    source: ryo_values::domain::plan_parts::types::ColumnIndex,
    destination: ryo_values::domain::plan_parts::types::ColumnIndex,
) -> crate::domain::closure::types::ReachParts {
    crate::domain::closure::types::ReachParts {
        keys: keys.to_vec(),
        source,
        destination,
    }
}
