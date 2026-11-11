//! The support of the `partitions` cases.
use super::super::super::PartitionedClosure;
use super::super::super::constants::LIVE_PARTITION_VIEW;
use super::super::super::fixtures::working_capacity;
use ryo_derived::domain::key::types::{ChainLink, HashedRow, KeyCellsHash};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::rows::types::RowIndex;

#[cfg(test)]
/// All live Value payload cells owned by this cursor, not only adjacency rows.
pub(super) fn working_cells(closure: &PartitionedClosure<'_>) -> usize {
    closure.current.as_ref().map_or(0, |_| {
        let current = closure.current().expect(LIVE_PARTITION_VIEW);
        current.table.cells.len() + current.key.len() + current.sources.len()
    }) + closure.key.len()
        + closure.scratch.row.len()
}

#[cfg(test)]
pub(super) fn ordering_bytes(closure: &PartitionedClosure<'_>) -> usize {
    let mut bytes = closure
        .ordering_capacity
        .0
        .saturating_mul(size_of::<RowIndex>())
        .saturating_add(working_capacity(closure).saturating_mul(size_of::<Value>()));
    if closure.current.is_some() {
        let current = closure.current().expect(LIVE_PARTITION_VIEW);
        let capacities = ryo_derived::domain::equality::service::capacities(&current.index);
        let [rows, hashes, heads, links, sorted, pairs, runs] = [
            capacities.rows.0,
            capacities.hashes.0,
            capacities.heads.0,
            capacities.links.0,
            capacities.sorted.0,
            capacities.pairs.0,
            capacities.runs.0,
        ];
        bytes = bytes
            .saturating_add(current.table.kinds.capacity() * size_of::<CellKind>())
            .saturating_add((rows + sorted) * size_of::<RowIndex>())
            .saturating_add(hashes * size_of::<KeyCellsHash>())
            .saturating_add((heads + links) * size_of::<ChainLink>())
            .saturating_add(pairs * size_of::<HashedRow>())
            .saturating_add(runs.saturating_mul(size_of::<KeyCellsHash>() + 1));
    }
    bytes
}

#[cfg(test)]
pub(super) fn scope() -> std::rc::Rc<crate::application::file_dag::relations::RelationOwner> {
    std::rc::Rc::new(crate::application::file_dag::relations::RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        crate::application::file_dag::fixtures::program_derived_port(),
    ))
}

#[cfg(test)]
pub(super) fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}
