//! Allocation facts only; the engine alone assigns lifetime classes and budgets.
use super::types::{BackingKind, PartitionEntry, PayloadFootprint, SharedBacking, VertexEntry};
use crate::domain::allocation::types::{AllocationBytes, AllocationKey, ElementCapacity};
use crate::domain::contract::types::BuildWorkspace;
use crate::domain::contract::types::derived_payload::value::{DerivedPayload, PayloadData};
use crate::domain::equality::types::KeyIndex;
use crate::domain::fault::types::DerivedFault;
use crate::domain::key::types::KeyCellsHash;
use crate::domain::membership::types::DerivedMembership;
use crate::domain::ordered::types::{OrderedText, OwnedOrderedKey, OwnedOrderedRow};
use crate::domain::sharing::service::{marks_capacity, marks_key, positions_capacity, positions_key};
use crate::domain::sharing::types::SharedPositions;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::rows::types::Table;
use std::mem::size_of;

fn sum(
    values: impl IntoIterator<Item = AllocationBytes>,
) -> Result<AllocationBytes, DerivedFault> {
    values.into_iter().try_fold(AllocationBytes(0), |a, b| {
        a.0.checked_add(b.0)
            .map(AllocationBytes)
            .ok_or(DerivedFault::ShapeMismatch)
    })
}
fn vector<T>(value: &Vec<T>) -> AllocationBytes {
    AllocationBytes(value.capacity().saturating_mul(size_of::<T>()))
}
fn hash(capacity: ElementCapacity, entry: AllocationBytes) -> AllocationBytes {
    if capacity.0 == 0 {
        AllocationBytes(0)
    } else {
        AllocationBytes(
            capacity
                .0
                .saturating_add(1)
                .checked_next_power_of_two()
                .unwrap_or(usize::MAX)
                .saturating_mul(entry.0 + 1)
                .saturating_add(64),
        )
    }
}
fn equality(value: &KeyIndex) -> Result<AllocationBytes, DerivedFault> {
    sum([
        vector(&value.rows),
        vector(&value.hashes),
        vector(&value.heads),
        vector(&value.next),
        vector(&value.sorted),
        vector(&value.pairs),
        hash(
            ElementCapacity(value.runs.capacity()),
            AllocationBytes(size_of::<KeyCellsHash>()),
        ),
    ])
}
fn table(value: &Table) -> Result<AllocationBytes, DerivedFault> {
    sum([vector(&value.kinds), vector(&value.cells)])
}
fn ordered(rows: &Vec<OwnedOrderedRow>) -> Result<AllocationBytes, DerivedFault> {
    let mut bytes = vector(rows);
    for row in rows {
        bytes = sum([bytes, vector(&row.keys)])?;
        for key in &row.keys {
            if let OwnedOrderedKey::Text(OrderedText(text)) = key {
                bytes = sum([bytes, AllocationBytes(text.capacity())])?;
            }
        }
    }
    Ok(bytes)
}
fn shared_model<T>(
    value: &std::sync::Arc<T>,
    tag: BackingKind,
    buffers: AllocationBytes,
) -> Result<SharedBacking, DerivedFault> {
    Ok(SharedBacking {
        tag,
        allocation_key: AllocationKey(std::sync::Arc::as_ptr(value) as usize),
        bytes: sum([
            AllocationBytes(size_of::<T>()),
            AllocationBytes(2 * size_of::<usize>()),
            buffers,
        ])?,
        position_capacity: ElementCapacity(0),
    })
}
fn positions(value: &SharedPositions) -> SharedBacking {
    SharedBacking {
        tag: BackingKind(1),
        allocation_key: positions_key(value),
        bytes: AllocationBytes(
            size_of::<Vec<u32>>()
                + 2 * size_of::<usize>()
                + positions_capacity(value).0 * size_of::<u32>(),
        ),
        position_capacity: positions_capacity(value),
    }
}

/// What a payload occupies, as allocation facts.
///
/// # Errors
/// A size that does not fit.
pub fn describe(payload: &DerivedPayload) -> Result<PayloadFootprint, DerivedFault> {
    let mut shared = Vec::new();
    let owned = match &payload.data {
        PayloadData::Equality(value) => {
            shared.push(shared_model(value, BackingKind(11), equality(value)?)?);
            AllocationBytes(0)
        }
        PayloadData::PredicateMatches(value) | PayloadData::DistinctRepresentatives(value) => {
            shared.push(positions(value));
            AllocationBytes(0)
        }
        PayloadData::CandidatePostings(values) => {
            for value in values {
                shared.push(positions(&value.rows));
            }
            sum(std::iter::once(vector(values))
                .chain(values.iter().map(|value| AllocationBytes(value.text.0.len()))))?
        }
        PayloadData::Membership(DerivedMembership::Dense(value)) => {
            shared.push(SharedBacking {
                tag: BackingKind(10),
                allocation_key: marks_key(value),
                bytes: AllocationBytes(
                    size_of::<Vec<bool>>()
                        + 2 * size_of::<usize>()
                        + marks_capacity(value).0 * size_of::<bool>(),
                ),
                position_capacity: ElementCapacity(0),
            });
            AllocationBytes(0)
        }
        PayloadData::Membership(DerivedMembership::Values(value)) => {
            hash(
                ElementCapacity(value.capacity()),
                AllocationBytes(size_of::<Value>()),
            )
        }
        PayloadData::PartitionOrder(value) => vector(value),
        PayloadData::RawKeyOrder(value) => {
            let buffers = sum([vector(&value.keys), vector(&value.order)])?;
            shared.push(shared_model(value, BackingKind(32), buffers)?);
            AllocationBytes(0)
        }
        PayloadData::ClosureAdjacency(value) => {
            let buffers = sum([
                vector(&value.key),
                table(&value.table)?,
                equality(&value.index)?,
                vector(&value.sources),
            ])?;
            shared.push(shared_model(value, BackingKind(23), buffers)?);
            AllocationBytes(0)
        }
        PayloadData::CycleAdjacency(value) => {
            let buffers = sum([
                vector(&value.values),
                vector(&value.starts),
                vector(&value.leaving),
            ])?;
            shared.push(shared_model(value, BackingKind(24), buffers)?);
            AllocationBytes(0)
        }
        PayloadData::OrderedKeys(value) => {
            let buffers = sum([ordered(&value.left)?, ordered(&value.right)?])?;
            shared.push(shared_model(value, BackingKind(25), buffers)?);
            AllocationBytes(0)
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
            shared.push(shared_model(value, BackingKind(26), buffers)?);
            AllocationBytes(0)
        }
        PayloadData::NativeLookup(_) => AllocationBytes(0),
        PayloadData::Adjacency(value) => {
            let buffers = {
                let base = sum([
                    vector(&value.partitions),
                    vector(&value.vertices),
                    vector(&value.vertex_partitions),
                    hash(
                        ElementCapacity(value.partition_ids.capacity()),
                        AllocationBytes(size_of::<PartitionEntry>()),
                    ),
                    hash(
                        ElementCapacity(value.vertex_ids.capacity()),
                        AllocationBytes(size_of::<VertexEntry>()),
                    ),
                ])?;
                let partitions = sum(value.partitions.iter().map(|part| {
                    AllocationBytes(part.len() * size_of::<Value>() + 2 * size_of::<usize>())
                }))?;
                let directions = sum((0..2).map(|direction| {
                    sum([
                        vector(&value.offsets[direction]),
                        vector(&value.neighbors[direction]),
                        value.forests[direction]
                            .as_ref()
                            .map_or(AllocationBytes(0), vector),
                    ])
                    .unwrap_or(AllocationBytes(usize::MAX))
                }))?;
                sum([base, partitions, directions])?
            };
            shared.push(shared_model(value, BackingKind(9), buffers)?);
            AllocationBytes(0)
        }
    };
    Ok(PayloadFootprint {
        owned_bytes: sum([
            AllocationBytes(size_of::<DerivedPayload>()),
            AllocationBytes(2 * size_of::<usize>()),
            owned,
        ])?,
        shared,
    })
}

/// The bytes of the scratch a build was lent.
///
/// # Errors
/// A size that does not fit.
pub fn working_bytes(room: &BuildWorkspace) -> Result<AllocationBytes, DerivedFault> {
    room.row
        .capacity()
        .checked_mul(size_of::<Value>())
        .map(AllocationBytes)
        .ok_or(DerivedFault::ShapeMismatch)
}
