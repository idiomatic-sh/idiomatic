//! Allocation facts only; the engine alone assigns lifetime classes and budgets.
use crate::domain::contract::types::*;
use ryo_values::domain::rows::types::Table;
use std::mem::size_of;
fn sum(values: impl IntoIterator<Item = usize>) -> Result<usize, DerivedFault> {
    values.into_iter().try_fold(0_usize, |a, b| {
        a.checked_add(b).ok_or(DerivedFault::ShapeMismatch)
    })
}
fn vector<T>(value: &Vec<T>) -> usize {
    value.capacity().saturating_mul(size_of::<T>())
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
fn equality(value: &KeyIndex) -> Result<usize, DerivedFault> {
    sum([
        vector(&value.rows),
        vector(&value.hashes),
        vector(&value.heads),
        vector(&value.next),
        vector(&value.sorted),
        vector(&value.pairs),
        hash::<KeyCellsHash>(value.runs.capacity()),
    ])
}
fn table(value: &Table) -> Result<usize, DerivedFault> {
    sum([vector(&value.kinds), vector(&value.cells)])
}
fn ordered(rows: &Vec<OwnedOrderedRow>) -> Result<usize, DerivedFault> {
    let mut bytes = vector(rows);
    for row in rows {
        bytes = sum([bytes, vector(&row.keys)])?;
        for key in &row.keys {
            if let OwnedOrderedKey::Text(text) = key {
                bytes = sum([bytes, text.capacity()])?;
            }
        }
    }
    Ok(bytes)
}
fn shared_model<T>(
    value: &std::sync::Arc<T>,
    tag: u8,
    buffers: usize,
) -> Result<SharedBacking, DerivedFault> {
    Ok(SharedBacking {
        tag,
        allocation_key: std::sync::Arc::as_ptr(value) as usize,
        bytes: sum([size_of::<T>(), 2 * size_of::<usize>(), buffers])?,
        position_capacity: 0,
    })
}
fn positions(value: &SharedPositions) -> SharedBacking {
    SharedBacking {
        tag: 1,
        allocation_key: value.allocation_key(),
        bytes: size_of::<Vec<u32>>() + 2 * size_of::<usize>() + value.capacity() * size_of::<u32>(),
        position_capacity: value.capacity(),
    }
}
pub(crate) fn describe(payload: &DerivedPayload) -> Result<PayloadFootprint, DerivedFault> {
    let mut shared = Vec::new();
    let owned = match &payload.data {
        PayloadData::Equality(value) => {
            shared.push(shared_model(value, 11, equality(value)?)?);
            0
        }
        PayloadData::PredicateMatches(value) | PayloadData::DistinctRepresentatives(value) => {
            shared.push(positions(value));
            0
        }
        PayloadData::CandidatePostings(values) => {
            for value in values {
                shared.push(positions(&value.rows));
            }
            sum(std::iter::once(vector(values))
                .chain(values.iter().map(|value| value.text.capacity())))?
        }
        PayloadData::Membership(Membership::Dense(value)) => {
            shared.push(SharedBacking {
                tag: 10,
                allocation_key: value.allocation_key(),
                bytes: size_of::<Vec<bool>>()
                    + 2 * size_of::<usize>()
                    + value.capacity() * size_of::<bool>(),
                position_capacity: 0,
            });
            0
        }
        PayloadData::Membership(Membership::Values(value)) => {
            hash::<ryo_relations::domain::relations::types::Value>(value.capacity())
        }
        PayloadData::PartitionOrder(value) => vector(value),
        PayloadData::RawKeyOrder(value) => {
            let buffers = sum([vector(&value.keys), vector(&value.order)])?;
            shared.push(shared_model(value, 32, buffers)?);
            0
        }
        PayloadData::ClosureAdjacency(value) => {
            let buffers = sum([
                vector(&value.key),
                table(&value.table)?,
                equality(&value.index)?,
                vector(&value.sources),
            ])?;
            shared.push(shared_model(value, 23, buffers)?);
            0
        }
        PayloadData::CycleAdjacency(value) => {
            let buffers = sum([
                vector(&value.values),
                vector(&value.starts),
                vector(&value.leaving),
            ])?;
            shared.push(shared_model(value, 24, buffers)?);
            0
        }
        PayloadData::OrderedKeys(value) => {
            let buffers = sum([ordered(&value.left)?, ordered(&value.right)?])?;
            shared.push(shared_model(value, 25, buffers)?);
            0
        }
        PayloadData::OrderedDfs(value) => {
            let buffers = sum([
                table(&value.roots)?,
                table(&value.edges)?,
                equality(&value.root_index)?,
                equality(&value.edge_index)?,
                vector(&value.root_columns),
                vector(&value.edge_columns),
                vector(&value.adjacency),
            ])?;
            shared.push(shared_model(value, 26, buffers)?);
            0
        }
        PayloadData::NativeLookup(_) => 0,
        PayloadData::Adjacency(value) => {
            let buffers = {
                use ryo_relations::domain::relations::types::Value;
                let base = sum([
                    vector(&value.partitions),
                    vector(&value.vertices),
                    vector(&value.vertex_partitions),
                    hash::<(std::sync::Arc<[Value]>, u32)>(value.partition_ids.capacity()),
                    hash::<((u32, Value), u32)>(value.vertex_ids.capacity()),
                ])?;
                let partitions = sum(value
                    .partitions
                    .iter()
                    .map(|part| part.len() * size_of::<Value>() + 2 * size_of::<usize>()))?;
                let directions = sum((0..2).map(|direction| {
                    sum([
                        vector(&value.offsets[direction]),
                        vector(&value.neighbors[direction]),
                        value.forests[direction].as_ref().map_or(0, vector),
                    ])
                    .unwrap_or(usize::MAX)
                }))?;
                sum([base, partitions, directions])?
            };
            shared.push(shared_model(value, 9, buffers)?);
            0
        }
    };
    Ok(PayloadFootprint {
        owned_bytes: sum([size_of::<DerivedPayload>(), 2 * size_of::<usize>(), owned])?,
        shared,
    })
}
