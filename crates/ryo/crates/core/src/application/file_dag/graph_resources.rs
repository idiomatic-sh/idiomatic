//! Engine-owned graph and ordered-key preparation; primitives only borrow these resources.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::graph::{
    ClosurePartition, CycleGraph, OrderedMatching, OwnedOrderedKey, OwnedOrderedRow,
};
use crate::domain::operator::contract::views::BorrowedRows;
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_groups::service::sort_rows;
use crate::domain::table_model::service::{cell, check_columns, index, push_row, row_in};
use crate::domain::table_model::types::{KeyIndex, On, Table};
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(crate) fn prepare_cycles(
    input: BorrowedRows<'_>,
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Result<CycleGraph, OperatingError> {
    check_columns(input.kinds(), &[source, destination])?;
    let mut graph = CycleGraph::default();
    let mut numbers = hashbrown::HashMap::new();
    let mut pairs = Vec::with_capacity(input.count());
    let mut row = Vec::new();
    for at in 0..input.count() {
        let Some(edge) = input.read(RowIndex(at), &mut row) else {
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

/// Each partition owns a projected endpoint table, exact hash index, and sorted sources.
pub(crate) fn prepare_closure(
    input: BorrowedRows<'_>,
    keys: &[ColumnIndex],
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Result<Vec<ClosurePartition>, OperatingError> {
    check_columns(input.kinds(), &[source, destination])?;
    check_columns(input.kinds(), keys)?;
    let endpoint_kinds = [
        *input
            .kinds()
            .get(source.0 as usize)
            .ok_or(OperatingError::ShapeMismatch)?,
        *input
            .kinds()
            .get(destination.0 as usize)
            .ok_or(OperatingError::ShapeMismatch)?,
    ];
    let mut table = Table {
        kinds: input.kinds().to_vec(),
        ..Table::default()
    };
    let mut buffer = Vec::new();
    for at in 0..input.count() {
        if let Some(row) = input.read(RowIndex(at), &mut buffer) {
            push_row(&mut table, row);
        }
    }
    let mut sorted = KeyIndex::default();
    sort_rows(&table, On::Columns(keys), &mut sorted)?;
    let mut partitions: Vec<ClosurePartition> = Vec::new();
    for position in &sorted.sorted {
        let edge = row_in(&table, *position).ok_or(OperatingError::ShapeMismatch)?;
        let key = keys
            .iter()
            .map(|column| cell(edge, *column))
            .collect::<Result<Vec<_>, _>>()?;
        if partitions.last().is_none_or(|last| last.key != key) {
            partitions.push(ClosurePartition {
                key,
                table: Table {
                    kinds: endpoint_kinds.to_vec(),
                    ..Table::default()
                },
                index: KeyIndex::default(),
                sources: Vec::new(),
            });
        }
        let partition = partitions.last_mut().ok_or(OperatingError::ShapeMismatch)?;
        let from = cell(edge, source)?;
        push_row(&mut partition.table, &[from, cell(edge, destination)?]);
        partition.sources.push(from);
    }
    for partition in &mut partitions {
        index(
            &partition.table,
            On::Columns(&[ColumnIndex(0)]),
            &mut partition.index,
        )?;
        partition.sources.sort_unstable();
        partition.sources.dedup();
    }
    Ok(partitions)
}

fn ordered_rows(
    input: BorrowedRows<'_>,
    keys: &[ColumnIndex],
    order: ColumnIndex,
    texts: &Texts<'_>,
) -> Result<Vec<OwnedOrderedRow>, OperatingError> {
    let mut rows = Vec::with_capacity(input.count());
    let mut buffer = Vec::new();
    for at in 0..input.count() {
        let Some(row) = input.read(RowIndex(at), &mut buffer) else {
            continue;
        };
        let Value::Number(order) = cell(row, order)? else {
            return Err(OperatingError::ShapeMismatch);
        };
        let group = keys
            .iter()
            .map(|column| {
                let value = cell(row, *column)?;
                Ok(match value {
                    Value::Symbol(_) | Value::Name(_) => {
                        let Datum::Text(text) = datum(value, texts) else {
                            return Err(OperatingError::ShapeMismatch);
                        };
                        OwnedOrderedKey::Text(text.0.to_owned())
                    }
                    _ => OwnedOrderedKey::Value(value),
                })
            })
            .collect::<Result<Vec<_>, OperatingError>>()?;
        rows.push(OwnedOrderedRow {
            keys: group,
            order,
            row: RowIndex(at),
        });
    }
    rows.sort_unstable();
    if rows.windows(2).any(|pair| matches!(pair, [first, second] if first.keys == second.keys && first.order == second.order)) {
        return Err(OperatingError::DuplicateOrder);
    }
    Ok(rows)
}

pub(crate) fn prepare_ordered(
    left: BorrowedRows<'_>,
    right: BorrowedRows<'_>,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    left_order: ColumnIndex,
    right_order: ColumnIndex,
    texts: &Texts<'_>,
) -> Result<OrderedMatching, OperatingError> {
    let kind = |input: BorrowedRows<'_>, column: ColumnIndex| {
        input.kinds().get(column.0 as usize).copied()
    };
    if left_on.len() != right_on.len()
        || kind(left, left_order) != Some(CellKind::Number)
        || kind(right, right_order) != Some(CellKind::Number)
    {
        return Err(OperatingError::ShapeMismatch);
    }
    for (left_key, right_key) in left_on.iter().zip(right_on) {
        if kind(left, *left_key) != kind(right, *right_key) {
            return Err(OperatingError::ShapeMismatch);
        }
    }
    Ok(OrderedMatching {
        left: ordered_rows(left, left_on, left_order, texts)?,
        right: ordered_rows(right, right_on, right_order, texts)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ryo_language::domain::relations::types::Integer;
    fn number(value: i64) -> Value {
        Value::Number(Integer(value))
    }
    fn table(width: usize, rows: &[&[i64]]) -> Table {
        let mut table = Table {
            kinds: vec![CellKind::Number; width],
            ..Table::default()
        };
        for row in rows {
            push_row(
                &mut table,
                &row.iter().copied().map(number).collect::<Vec<_>>(),
            );
        }
        table
    }
    #[test]
    fn adjacency_keeps_parallel_edges_and_first_entity_numbering() {
        let input = table(2, &[&[9, 1], &[9, 1], &[1, 9], &[7, 7]]);
        let graph = prepare_cycles(
            BorrowedRows::Computed(&input),
            ColumnIndex(0),
            ColumnIndex(1),
        )
        .unwrap();
        assert_eq!(graph.values, vec![number(9), number(1), number(7)]);
        assert_eq!(graph.starts, vec![0, 2, 3, 4]);
        assert_eq!(graph.leaving, vec![1, 1, 0, 2]);
    }
    #[test]
    fn closure_preparation_preserves_clustered_partition_order_and_sorts_sources() {
        let input = table(3, &[&[9, 4, 3], &[9, 2, 4], &[1, 7, 8]]);
        let partitions = prepare_closure(
            BorrowedRows::Computed(&input),
            &[ColumnIndex(0)],
            ColumnIndex(1),
            ColumnIndex(2),
        )
        .unwrap();
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].key, vec![number(9)]);
        assert_eq!(partitions[0].sources, vec![number(2), number(4)]);
        assert_eq!(
            partitions[0].table.cells,
            vec![number(4), number(3), number(2), number(4)]
        );
        assert_eq!(partitions[1].key, vec![number(1)]);
    }
}

mod dfs;
pub(crate) use dfs::prepare_dfs;
