//! Existing dense cycle adjacency, preserving first-seen IDs and duplicate edges.
use crate::domain::contract::types::{CycleGraph, DerivedFault, RowCells};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
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
pub(crate) fn build(
    input: &dyn RowCells,
    source: ColumnIndex,
    destination: ColumnIndex,
    row: &mut Vec<Value>,
) -> Result<CycleGraph, DerivedFault> {
    check_columns(input.kinds(), &[source, destination])?;
    let mut graph = CycleGraph::default();
    let mut numbers = hashbrown::HashMap::new();
    let mut pairs = Vec::with_capacity(input.count());
    for at in 0..input.count() {
        let Some(edge) = input.read_row(at, row) else {
            continue;
        };
        let mut number = |value: Value| {
            *numbers.entry(value).or_insert_with(|| {
                graph.values.push(value);
                graph.values.len() - 1
            })
        };
        pairs.push((
            number(cell(edge, source)?),
            number(cell(edge, destination)?),
        ));
    }
    graph.starts.resize(graph.values.len() + 1, 0);
    for (from, _) in &pairs {
        *graph
            .starts
            .get_mut(from + 1)
            .ok_or(DerivedFault::ShapeMismatch)? += 1;
    }
    for at in 0..graph.values.len() {
        let before = *graph.starts.get(at).ok_or(DerivedFault::ShapeMismatch)?;
        *graph
            .starts
            .get_mut(at + 1)
            .ok_or(DerivedFault::ShapeMismatch)? += before;
    }
    let mut filled = graph.starts.clone();
    graph.leaving.resize(pairs.len(), 0);
    for (from, to) in pairs {
        let place = filled.get_mut(from).ok_or(DerivedFault::ShapeMismatch)?;
        *graph
            .leaving
            .get_mut(*place)
            .ok_or(DerivedFault::ShapeMismatch)? = to;
        *place += 1;
    }
    Ok(graph)
}
