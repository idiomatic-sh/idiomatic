//! Existing compact adjacency algorithm; generation identity and retention are engine-owned.
use super::types::compact_graph::value::CompactGraph;
use crate::domain::fault::types::DerivedFault;
use crate::domain::row_cells::types::RowCells;
use crate::domain::vertices::constants::FOREST_ROOT;
use crate::domain::vertices::types::{
    NeighborList, OffsetList, ParentList, PartitionId, PartitionList, VertexId, VertexKey,
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;
use std::sync::Arc;

fn check_columns(
    kinds: &[ryo_relations::domain::relations::types::CellKind],
    columns: &[ColumnIndex],
) -> Result<(), DerivedFault> {
    for column in columns {
        if usize::try_from(column.0).map_or(true, |at| at >= kinds.len()) {
            return Err(DerivedFault::NoSuchColumn { column: *column });
        }
    }
    Ok(())
}

/// Builds the compact graph of the rows, partitioned by the key columns; absent when it does not fit the id space.
#[expect(
    clippy::as_conversions,
    reason = "a u32 offset or vertex number indexes the graph's own arrays: lossless on every target ryo builds for, and this per-edge kernel keeps its indexing panic instead of a refusal branch in every visit"
)]
pub(crate) fn build_borrowed(
    rows: &dyn RowCells,
    endpoints: [ColumnIndex; 2],
    keys: &[ColumnIndex],
) -> Result<Option<CompactGraph>, DerivedFault> {
    check_columns(rows.kinds(), &endpoints)?;
    check_columns(rows.kinds(), keys)?;
    if !u32::try_from(rows.count().0).is_ok_and(|count| count <= u32::MAX - 2) {
        return Ok(None);
    }
    let mut graph = CompactGraph {
        partitions: Vec::new(),
        partition_ids: Default::default(),
        vertices: Vec::new(),
        vertex_partitions: PartitionList(Vec::new()),
        vertex_ids: Default::default(),
        offsets: [OffsetList(Vec::new()), OffsetList(Vec::new())],
        neighbors: [NeighborList(Vec::new()), NeighborList(Vec::new())],
        forests: [None, None],
    };
    let mut pairs = Vec::new();
    let mut tuple = Vec::with_capacity(keys.len());
    for position in 0..rows.count().0 {
        let position = RowIndex(position);
        tuple.clear();
        let mut complete = true;
        for column in keys {
            if let Some(value) = rows.cell(position, *column) {
                tuple.push(value);
            } else {
                complete = false;
                break;
            }
        }
        if !complete {
            continue;
        }
        let partition = if let Some(id) = graph.partition_ids.get(tuple.as_slice()) {
            *id
        } else {
            let Ok(id) = u32::try_from(graph.partitions.len()) else {
                return Ok(None);
            };
            let key: Arc<[Value]> = Arc::from(tuple.as_slice());
            graph.partitions.push(Arc::clone(&key));
            let _ = graph.partition_ids.insert(key, PartitionId(id));
            PartitionId(id)
        };
        let mut vertices = [VertexId(0); 2];
        for (slot, column) in vertices.iter_mut().zip(endpoints) {
            let Some(value) = rows.cell(position, column) else {
                complete = false;
                break;
            };
            *slot = if let Some(vertex) = graph.vertex_ids.get(&VertexKey { partition, value }) {
                *vertex
            } else {
                let Ok(id) = u32::try_from(graph.vertices.len()) else {
                    return Ok(None);
                };
                if id >= u32::MAX - 2 {
                    return Ok(None);
                }
                let id = VertexId(id);
                graph.vertices.push(value);
                graph.vertex_partitions.0.push(partition.0);
                let _ = graph.vertex_ids.insert(VertexKey { partition, value }, id);
                id
            };
        }
        if complete {
            pairs.push(vertices);
        }
    }
    for orientation in 0..2 {
        let mut offsets = vec![0_u32; graph.vertices.len() + 1];
        for pair in &pairs {
            offsets[usize::from(pair[orientation]) + 1] += 1;
        }
        for vertex in 1..offsets.len() {
            offsets[vertex] += offsets[vertex - 1];
        }
        let mut written = offsets.clone();
        let mut neighbors = vec![0_u32; pairs.len()];
        for pair in &pairs {
            let source = usize::from(pair[orientation]);
            neighbors[written[source] as usize] = pair[1 - orientation].0;
            written[source] += 1;
        }
        let offsets = OffsetList(offsets);
        let neighbors = NeighborList(neighbors);
        graph.forests[orientation] = forest(&offsets, &neighbors);
        graph.offsets[orientation] = offsets;
        graph.neighbors[orientation] = neighbors;
    }
    Ok(Some(graph))
}

/// Validate duplicate-insensitive single-parent ancestry and cycles iteratively once per graph.
#[expect(
    clippy::as_conversions,
    reason = "a u32 offset or vertex number indexes the graph's own arrays: lossless on every target ryo builds for, and this per-edge kernel keeps its indexing panic instead of a refusal branch in every visit"
)]
fn forest(offsets: &OffsetList, neighbors: &NeighborList) -> Option<ParentList> {
    let (offsets, neighbors) = (&offsets.0, &neighbors.0);
    let mut parents = vec![FOREST_ROOT; offsets.len().saturating_sub(1)];
    for vertex in 0..parents.len() {
        let outgoing = &neighbors[offsets[vertex] as usize..offsets[vertex + 1] as usize];
        if let Some(parent) = outgoing.first() {
            if outgoing.iter().any(|other| other != parent) {
                return None;
            }
            parents[vertex] = *parent;
        }
    }
    let mut state = vec![0_u8; parents.len()];
    let mut path = Vec::new();
    for start in 0..parents.len() {
        if state[start] != 0 {
            continue;
        }
        path.clear();
        let mut current = start;
        loop {
            if state[current] == 1 {
                return None;
            }
            if state[current] == 2 {
                break;
            }
            state[current] = 1;
            path.push(current);
            if parents[current] == FOREST_ROOT {
                break;
            }
            current = parents[current] as usize;
        }
        for vertex in &path {
            state[*vertex] = 2;
        }
    }
    Some(ParentList(parents))
}
