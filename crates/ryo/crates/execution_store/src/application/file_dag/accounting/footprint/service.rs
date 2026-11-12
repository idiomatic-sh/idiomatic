//! What a built derived payload occupies, as allocation facts; the engine alone assigns lifetime classes and budgets.
use super::types::{BackingKind, PayloadFootprint, SharedBacking};
use ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph;
use ryo_derived::domain::allocation::types::{AllocationBytes, AllocationKey, ElementCapacity};
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::{BuildWorkspace, DerivedView};
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::key::types::KeyCellsHash;
use ryo_derived::domain::membership::types::MembershipView;
use ryo_derived::domain::ordered::types::{OrderedText, OwnedOrderedKey, OwnedOrderedRow};
use ryo_derived::domain::vertices::types::{EdgeDirection, PartitionId, VertexId, VertexKey};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::rows::types::{RowIndex, Table};

fn sum(values: impl IntoIterator<Item = AllocationBytes>) -> Result<AllocationBytes, DerivedFault> {
    values.into_iter().try_fold(AllocationBytes(0), |a, b| {
        a.0.checked_add(b.0)
            .map(AllocationBytes)
            .ok_or(DerivedFault::ShapeMismatch)
    })
}
fn vector<T>(value: &Vec<T>) -> AllocationBytes {
    elements::<T>(ElementCapacity(value.capacity()))
}
fn elements<T>(capacity: ElementCapacity) -> AllocationBytes {
    AllocationBytes(capacity.0.saturating_mul(size_of::<T>()))
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
/// The allocation an `Arc<T>` holds `value` in: its counts, the value and its buffers.
fn shared_model<T>(
    value: &T,
    tag: BackingKind,
    buffers: AllocationBytes,
) -> Result<SharedBacking, DerivedFault> {
    Ok(SharedBacking {
        tag,
        allocation_key: AllocationKey(std::ptr::from_ref(value).addr()),
        bytes: sum([
            AllocationBytes(size_of::<T>()),
            AllocationBytes(2 * size_of::<usize>()),
            buffers,
        ])?,
        position_capacity: ElementCapacity(0),
    })
}
fn graph(value: &CompactGraph) -> Result<AllocationBytes, DerivedFault> {
    let capacities = value.capacities();
    let base = sum([
        elements::<std::sync::Arc<[Value]>>(capacities.partitions),
        elements::<Value>(capacities.vertices),
        vector(&value.vertex_partitions().0),
        hash(
            capacities.partition_ids,
            // One entry of the partition table, as the hash table stores it.
            AllocationBytes(size_of::<(std::sync::Arc<[Value]>, PartitionId)>()),
        ),
        hash(
            capacities.vertex_ids,
            // One entry of the vertex table, as the hash table stores it.
            AllocationBytes(size_of::<(VertexKey, VertexId)>()),
        ),
    ])?;
    let partitions = sum([
        elements::<Value>(capacities.partition_values),
        elements::<[usize; 2]>(capacities.partition_count),
    ])?;
    let directions = sum([EdgeDirection::Forward, EdgeDirection::Backward]
        .into_iter()
        .zip(capacities.forests)
        .map(|(direction, forest)| {
            sum([
                vector(&value.offsets(direction).0),
                vector(&value.neighbors(direction).0),
                forest.map_or(AllocationBytes(0), elements::<u32>),
            ])
            .unwrap_or(AllocationBytes(usize::MAX))
        }))?;
    sum([base, partitions, directions])
}

/// What a payload occupies, as allocation facts, read through the payload's own view.
///
/// # Errors
/// A size that does not fit.
pub(crate) fn describe(payload: &DerivedPayload) -> Result<PayloadFootprint, DerivedFault> {
    let view = ryo_derived::domain::dispatch::service::payload_view(payload);
    let mut shared = Vec::new();
    let owned = match &view {
        DerivedView::Equality(value) => {
            shared.push(shared_model(*value, BackingKind(11), equality(value)?)?);
            AllocationBytes(0)
        }
        DerivedView::Membership(MembershipView::Dense(_)) => {
            let marks = payload.share_marks().ok_or(DerivedFault::ShapeMismatch)?;
            shared.push(SharedBacking {
                tag: BackingKind(10),
                allocation_key: AllocationKey(std::sync::Arc::as_ptr(&marks.0).addr()),
                bytes: sum([
                    AllocationBytes(size_of::<Vec<bool>>() + 2 * size_of::<usize>()),
                    elements::<bool>(ElementCapacity(marks.0.capacity())),
                ])?,
                position_capacity: ElementCapacity(0),
            });
            AllocationBytes(0)
        }
        DerivedView::Membership(MembershipView::Values(value)) => hash(
            ElementCapacity(value.capacity()),
            AllocationBytes(size_of::<Value>()),
        ),
        DerivedView::PartitionOrder(_) => elements::<RowIndex>(
            payload
                .partition_order_capacity()
                .ok_or(DerivedFault::ShapeMismatch)?,
        ),
        DerivedView::ClosureAdjacency(value) => {
            let buffers = sum([
                vector(&value.key),
                table(&value.table)?,
                equality(&value.index)?,
                vector(&value.sources),
            ])?;
            shared.push(shared_model(*value, BackingKind(23), buffers)?);
            AllocationBytes(0)
        }
        DerivedView::CycleAdjacency(value) => {
            let buffers = sum([
                vector(&value.values),
                vector(&value.starts.0),
                vector(&value.leaving.0),
            ])?;
            shared.push(shared_model(*value, BackingKind(24), buffers)?);
            AllocationBytes(0)
        }
        DerivedView::OrderedKeys(value) => {
            let buffers = sum([ordered(&value.left)?, ordered(&value.right)?])?;
            shared.push(shared_model(*value, BackingKind(25), buffers)?);
            AllocationBytes(0)
        }
        DerivedView::OrderedDfs(value) => {
            let buffers = sum([
                table(&value.roots)?,
                table(&value.edges)?,
                equality(&value.root_index)?,
                equality(&value.edge_index)?,
                vector(&value.root_columns),
                vector(&value.edge_columns),
                vector(&value.adjacency),
            ])?;
            shared.push(shared_model(*value, BackingKind(26), buffers)?);
            AllocationBytes(0)
        }
        DerivedView::NativeLookup(_) => AllocationBytes(0),
        DerivedView::Adjacency(value) => {
            shared.push(shared_model(*value, BackingKind(9), graph(value)?)?);
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
pub(crate) fn working_bytes(room: &BuildWorkspace) -> Result<AllocationBytes, DerivedFault> {
    room.row
        .capacity()
        .checked_mul(size_of::<Value>())
        .map(AllocationBytes)
        .ok_or(DerivedFault::ShapeMismatch)
}
