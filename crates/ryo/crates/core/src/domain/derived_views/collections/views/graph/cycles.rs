//! Dense adjacency for cycle traversal, preserving duplicate edges and first-seen IDs.
use crate::domain::{
    operating::errors::OperatingError,
    operator::contract::{graph::CycleGraph, views::BorrowedRows},
    table_model::service::{cell, check_columns},
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(super) fn prepare(
    input: BorrowedRows<'_>,
    source: ColumnIndex,
    destination: ColumnIndex,
    row: &mut Vec<Value>,
) -> Result<CycleGraph, OperatingError> {
    check_columns(input.kinds(), &[source, destination])?;
    let mut graph = CycleGraph::default();
    let mut numbers = hashbrown::HashMap::new();
    let mut pairs = Vec::with_capacity(input.count());
    for at in 0..input.count() {
        let Some(edge) = input.read(RowIndex(at), row) else {
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
            .ok_or(OperatingError::ShapeMismatch)? += 1;
    }
    for at in 0..graph.values.len() {
        let before = *graph.starts.get(at).ok_or(OperatingError::ShapeMismatch)?;
        *graph
            .starts
            .get_mut(at + 1)
            .ok_or(OperatingError::ShapeMismatch)? += before;
    }
    let mut filled = graph.starts.clone();
    graph.leaving.resize(pairs.len(), 0);
    for (from, to) in pairs {
        let place = filled.get_mut(from).ok_or(OperatingError::ShapeMismatch)?;
        *graph
            .leaving
            .get_mut(*place)
            .ok_or(OperatingError::ShapeMismatch)? = to;
        *place += 1;
    }
    Ok(graph)
}

use ryo_values::domain::rows::types::RowIndex;
