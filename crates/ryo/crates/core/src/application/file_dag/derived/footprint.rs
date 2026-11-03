//! Charge owned capacities, including required views refused optional retention.
use crate::domain::{
    derived_views::types::*,
    operating::errors::OperatingError,
    table_model::types::{KeyIndex, Table},
};
fn vector<T>(values: &Vec<T>) -> usize {
    values.capacity().saturating_mul(size_of::<T>())
}
fn hash<T>(capacity: usize) -> usize {
    if capacity == 0 {
        0
    } else {
        capacity
            .saturating_add(1)
            .checked_next_power_of_two()
            .unwrap_or(usize::MAX)
            .saturating_mul(size_of::<T>() + 1)
            .saturating_add(64)
    }
}
fn index(index: &KeyIndex) -> usize {
    vector(&index.rows)
        + vector(&index.hashes)
        + vector(&index.heads)
        + vector(&index.next)
        + vector(&index.sorted)
        + vector(&index.pairs)
        + hash::<crate::domain::table_model::types::KeyCellsHash>(index.runs.capacity())
}
fn table(table: &Table) -> usize {
    vector(&table.kinds) + vector(&table.cells)
}
fn ordered(rows: &Vec<crate::domain::operator::contract::graph::OwnedOrderedRow>) -> usize {
    use crate::domain::operator::contract::graph::OwnedOrderedKey;
    vector(rows)
        + rows
            .iter()
            .map(|row| {
                vector(&row.keys)
                    + row
                        .keys
                        .iter()
                        .map(|key| match key {
                            OwnedOrderedKey::Text(text) => text.capacity(),
                            OwnedOrderedKey::Value(_) => 0,
                        })
                        .sum::<usize>()
            })
            .sum::<usize>()
}
pub(super) fn storage_bytes(storage: &DerivedStorage) -> Result<usize, OperatingError> {
    use ryo_relations::domain::relations::types::Value;
    let payload = match storage {
        DerivedStorage::Unbuilt => return Err(OperatingError::NotRun),
        DerivedStorage::Equality(data) => index(data),
        DerivedStorage::EqualityShared(_) => 0,
        DerivedStorage::RowIds { rows, .. } => vector(rows),
        DerivedStorage::Postings(rows) => {
            vector(rows)
                + rows
                    .iter()
                    .map(|row| {
                        row.text.capacity()
                            + vector(&row.rows)
                            + size_of::<Vec<u32>>()
                            + 2 * size_of::<usize>()
                    })
                    .sum::<usize>()
        }
        DerivedStorage::Membership(MembershipData::Values(values)) => {
            hash::<Value>(values.capacity())
        }
        DerivedStorage::Membership(MembershipData::Dense(values)) => vector(values),
        DerivedStorage::PartitionOrder(rows) => vector(rows),
        DerivedStorage::ClosureAdjacency(part) => {
            vector(&part.key) + table(&part.table) + index(&part.index) + vector(&part.sources)
        }
        DerivedStorage::CycleAdjacency(graph) => {
            vector(&graph.values) + vector(&graph.starts) + vector(&graph.leaving)
        }
        DerivedStorage::OrderedKeys(keys) => ordered(&keys.left) + ordered(&keys.right),
        DerivedStorage::OrderedDfs(graph) => {
            table(&graph.roots)
                + table(&graph.edges)
                + index(&graph.root_index)
                + index(&graph.edge_index)
                + vector(&graph.layout.edge_columns)
                + vector(&graph.layout.root_columns)
                + vector(&graph.layout.adjacency)
        }
        DerivedStorage::NativeLookup(_) => 0,
        DerivedStorage::Adjacency(graph) => {
            let mut bytes = vector(&graph.partitions)
                + vector(&graph.vertices)
                + vector(&graph.vertex_partitions)
                + hash::<(std::sync::Arc<[Value]>, u32)>(graph.partition_ids.capacity())
                + hash::<((u32, Value), u32)>(graph.vertex_ids.capacity())
                + vector(&graph.identity.keys);
            bytes += graph
                .partitions
                .iter()
                .map(|part| part.len() * size_of::<Value>() + 2 * size_of::<usize>())
                .sum::<usize>();
            bytes += graph
                .identity
                .population
                .positions
                .as_ref()
                .map_or(0, |rows| {
                    vector(rows) + size_of::<Vec<u32>>() + 2 * size_of::<usize>()
                });
            for direction in 0..2 {
                bytes += vector(&graph.offsets[direction])
                    + vector(&graph.neighbors[direction])
                    + graph.forests[direction].as_ref().map_or(0, vector);
            }
            bytes
        }
    };
    payload
        .checked_add(size_of::<DerivedStorage>() + 2 * size_of::<usize>())
        .ok_or(OperatingError::ShapeMismatch)
}
