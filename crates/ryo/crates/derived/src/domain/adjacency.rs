//! Existing compact adjacency algorithm; generation identity and retention are engine-owned.
use super::contract::types::{CompactGraph, DerivedFault, FOREST_ROOT, RowCells};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
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
pub(crate) fn build_borrowed(
    rows: &dyn RowCells,
    endpoints: [ColumnIndex; 2],
    keys: &[ColumnIndex],
) -> Result<Option<CompactGraph>, DerivedFault> {
    check_columns(rows.kinds(), &endpoints)?;
    check_columns(rows.kinds(), keys)?;
    if rows.count() > (u32::MAX - 2) as usize {
        return Ok(None);
    }
    let mut graph = CompactGraph {
        partitions: Vec::new(),
        partition_ids: Default::default(),
        vertices: Vec::new(),
        vertex_partitions: Vec::new(),
        vertex_ids: Default::default(),
        offsets: [Vec::new(), Vec::new()],
        neighbors: [Vec::new(), Vec::new()],
        forests: [None, None],
    };
    let mut pairs = Vec::new();
    let mut tuple = Vec::with_capacity(keys.len());
    for position in 0..rows.count() {
        let position = position;
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
            let _ = graph.partition_ids.insert(key, id);
            id
        };
        let mut vertices = [0_u32; 2];
        for (slot, column) in vertices.iter_mut().zip(endpoints) {
            let Some(value) = rows.cell(position, column) else {
                complete = false;
                break;
            };
            *slot = if let Some(vertex) = graph.vertex_ids.get(&(partition, value)) {
                *vertex
            } else {
                if graph.vertices.len() >= (u32::MAX - 2) as usize {
                    return Ok(None);
                }
                let id = u32::try_from(graph.vertices.len()).unwrap_or(u32::MAX);
                graph.vertices.push(value);
                graph.vertex_partitions.push(partition);
                let _ = graph.vertex_ids.insert((partition, value), id);
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
            offsets[pair[orientation] as usize + 1] += 1;
        }
        for vertex in 1..offsets.len() {
            offsets[vertex] += offsets[vertex - 1];
        }
        let mut written = offsets.clone();
        let mut neighbors = vec![0_u32; pairs.len()];
        for pair in &pairs {
            let source = pair[orientation] as usize;
            neighbors[written[source] as usize] = pair[1 - orientation];
            written[source] += 1;
        }
        graph.forests[orientation] = forest(&offsets, &neighbors);
        graph.offsets[orientation] = offsets;
        graph.neighbors[orientation] = neighbors;
    }
    Ok(Some(graph))
}

/// Validate duplicate-insensitive single-parent ancestry and cycles iteratively once per graph.
fn forest(offsets: &[u32], neighbors: &[u32]) -> Option<Vec<u32>> {
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
    Some(parents)
}
