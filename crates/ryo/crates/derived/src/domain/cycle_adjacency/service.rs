//! Existing dense cycle adjacency, preserving first-seen IDs and duplicate edges.
use super::types::{CycleGraph, LeavingStart, NodeSlot};
use crate::domain::fault::types::DerivedFault;
use crate::domain::row_cells::types::RowCells;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

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

fn cell(row: &[Value], column: ColumnIndex) -> Result<Value, DerivedFault> {
    row.get(column.0 as usize)
        .copied()
        .ok_or(DerivedFault::NoSuchColumn { column })
}

/// Numbers the nodes in first-seen order and groups each edge under its source.
pub(crate) fn build(
    input: &dyn RowCells,
    source: ColumnIndex,
    destination: ColumnIndex,
    row: &mut Vec<Value>,
) -> Result<CycleGraph, DerivedFault> {
    check_columns(input.kinds(), &[source, destination])?;
    let mut graph = CycleGraph::default();
    let mut numbers = hashbrown::HashMap::new();
    let mut pairs = Vec::with_capacity(input.count().0);
    for at in 0..input.count().0 {
        let Some(edge) = input.read_row(RowIndex(at), row) else {
            continue;
        };
        let mut number = |value: Value| {
            *numbers.entry(value).or_insert_with(|| {
                graph.values.push(value);
                NodeSlot(graph.values.len() - 1)
            })
        };
        pairs.push((
            number(cell(edge, source)?),
            number(cell(edge, destination)?),
        ));
    }
    graph.starts.resize(graph.values.len() + 1, LeavingStart(0));
    for (from, _) in &pairs {
        graph
            .starts
            .get_mut(from.0 + 1)
            .ok_or(DerivedFault::ShapeMismatch)?
            .0 += 1;
    }
    for at in 0..graph.values.len() {
        let before = graph.starts.get(at).ok_or(DerivedFault::ShapeMismatch)?.0;
        graph
            .starts
            .get_mut(at + 1)
            .ok_or(DerivedFault::ShapeMismatch)?
            .0 += before;
    }
    let mut filled = graph.starts.clone();
    graph.leaving.resize(pairs.len(), NodeSlot(0));
    for (from, to) in pairs {
        let place = filled.get_mut(from.0).ok_or(DerivedFault::ShapeMismatch)?;
        *graph
            .leaving
            .get_mut(place.0)
            .ok_or(DerivedFault::ShapeMismatch)? = to;
        place.0 += 1;
    }
    Ok(graph)
}
