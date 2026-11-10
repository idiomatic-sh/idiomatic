//! Ordered DFS lookup preparation; traversal and completion admission remain engine-owned.
use super::types::{DfsColumns, EdgeRun, EdgeSlot, PreparedDfs};
use crate::domain::fault::types::DerivedFault;
use crate::domain::row_cells::types::{CanonicalTexts, RowCells};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::RowIndex;
use ryo_values::domain::rows::types::Table;
fn check(input: &dyn RowCells, columns: &[ColumnIndex]) -> Result<(), DerivedFault> {
    for column in columns {
        if usize::try_from(column.0).map_or(true, |at| at >= input.kinds().len()) {
            return Err(DerivedFault::NoSuchColumn { column: *column });
        }
    }
    Ok(())
}
fn materialize(input: &dyn RowCells, buffer: &mut Vec<Value>) -> Table {
    let mut table = Table {
        kinds: input.kinds().to_vec(),
        ..Table::default()
    };
    for row in 0..input.count().0 {
        if let Some(cells) = input.read_row(RowIndex(row), buffer) {
            table.cells.extend_from_slice(cells);
            table.count.0 += 1;
        }
    }
    table
}
fn canonical_columns(
    table: &mut Table,
    columns: &mut [ColumnIndex],
    texts: &mut dyn CanonicalTexts,
) -> Result<(), DerivedFault> {
    let width = table.kinds.len();
    let after = width
        .checked_add(columns.len())
        .ok_or(DerivedFault::ShapeMismatch)?;
    let cells = after
        .checked_mul(table.count.0)
        .ok_or(DerivedFault::ShapeMismatch)?;
    let _addressable_width = u32::try_from(after).map_err(|_| DerivedFault::ShapeMismatch)?;
    for column in columns.iter() {
        table.kinds.push(
            *table
                .kinds
                .get(usize::from(*column))
                .ok_or(DerivedFault::ShapeMismatch)?,
        );
    }
    table.cells.resize(
        cells,
        Value::Number(ryo_relations::domain::relations::types::Integer(0)),
    );
    let mut buffer = String::new();
    for row in (0..table.count.0).rev() {
        table
            .cells
            .copy_within(row * width..(row + 1) * width, row * after);
        for (place, column) in columns.iter().enumerate() {
            let index = usize::try_from(column.0).map_err(|_| DerivedFault::ShapeMismatch)?;
            let at = row
                .checked_mul(after)
                .and_then(|row| row.checked_add(index))
                .ok_or(DerivedFault::ShapeMismatch)?;
            let value = *table.cells.get(at).ok_or(DerivedFault::ShapeMismatch)?;
            let found = match value {
                Value::Symbol(_) => texts.text(value),
                Value::Name(_) => Some(texts.text(value).ok_or(DerivedFault::ShapeMismatch)?),
                Value::Node(_)
                | Value::File(_)
                | Value::Span(_)
                | Value::Number(_)
                | Value::Flag(_) => None,
            };
            let canonical = if let Some(text) = found {
                buffer.clear();
                buffer.push_str(text.0);
                texts.intern(CellText(&buffer))?
            } else {
                value
            };
            let at = row
                .checked_mul(after)
                .and_then(|row| row.checked_add(width))
                .and_then(|row| row.checked_add(place))
                .ok_or(DerivedFault::ShapeMismatch)?;
            *table.cells.get_mut(at).ok_or(DerivedFault::ShapeMismatch)? = canonical;
        }
    }
    for (place, column) in columns.iter_mut().enumerate() {
        *column =
            ColumnIndex(u32::try_from(width + place).map_err(|_| DerivedFault::ShapeMismatch)?);
    }
    Ok(())
}
/// Prepares the roots and edges of an ordered DFS lookup.
pub(crate) fn build(
    columns: DfsColumns<'_>,
    roots: &dyn RowCells,
    edges: &dyn RowCells,
    texts: &mut dyn CanonicalTexts,
    buffer: &mut Vec<Value>,
) -> Result<PreparedDfs, DerivedFault> {
    let DfsColumns {
        root,
        root_keys,
        source,
        destination,
        order,
        edge,
        edge_keys,
    } = columns;
    check(roots, &[root])?;
    check(roots, root_keys)?;
    check(edges, &[source, destination, order, edge])?;
    check(edges, edge_keys)?;
    let kind = roots
        .kinds()
        .get(usize::from(root))
        .ok_or(DerivedFault::ShapeMismatch)?;
    if Some(kind)
        != edges
            .kinds()
            .get(usize::from(source))
        || Some(kind)
            != edges
                .kinds()
                .get(usize::from(destination))
        || edges
            .kinds()
            .get(usize::from(order))
            != Some(&CellKind::Number)
        || root_keys.len() != edge_keys.len()
        || root_keys.iter().zip(edge_keys).any(|(left, right)| {
            roots
                .kinds()
                .get(usize::from(*left))
                != edges
                    .kinds()
                    .get(usize::from(*right))
        })
    {
        return Err(DerivedFault::ShapeMismatch);
    }
    let mut roots = materialize(roots, buffer);
    let mut edges = materialize(edges, buffer);
    let mut root_columns = root_keys.to_vec();
    root_columns.push(root);
    let mut edge_columns = edge_keys.to_vec();
    edge_columns.extend([source, destination]);
    canonical_columns(&mut roots, &mut root_columns, texts)?;
    canonical_columns(&mut edges, &mut edge_columns, texts)?;
    let from = edge_columns
        .get(..edge_columns.len().saturating_sub(1))
        .unwrap_or_default();
    for at in 0..edges.count.0 {
        if !matches!(edges.cell(RowIndex(at), order), Some(Value::Number(_))) {
            return Err(DerivedFault::ShapeMismatch);
        }
    }
    let mut edge_order = crate::domain::partition::service::ordered_positions(&edges, from)?;
    let mut adjacency = vec![
        EdgeRun {
            start: EdgeSlot(0),
            end: EdgeSlot(0)
        };
        edges.count.0
    ];
    let mut begin = 0;
    while begin < edge_order.len() {
        let head = *edge_order.get(begin).ok_or(DerivedFault::ShapeMismatch)?;
        let mut end = begin + 1;
        while end < edge_order.len() {
            let next = *edge_order.get(end).ok_or(DerivedFault::ShapeMismatch)?;
            if !from
                .iter()
                .all(|column| edges.cell(head, *column) == edges.cell(next, *column))
            {
                break;
            }
            end += 1;
        }
        let group = edge_order
            .get_mut(begin..end)
            .ok_or(DerivedFault::ShapeMismatch)?;
        group.sort_unstable_by_key(|at| edges.cell(*at, order));
        if group
            .windows(2)
            .any(|pair| edges.cell(pair[0], order) == edges.cell(pair[1], order))
        {
            return Err(DerivedFault::DuplicateDfsOrder);
        }
        for row in group {
            *adjacency
                .get_mut(row.0)
                .ok_or(DerivedFault::ShapeMismatch)? = EdgeRun {
                start: EdgeSlot(begin),
                end: EdgeSlot(end),
            };
        }
        begin = end;
    }
    let mut edge_index = crate::domain::equality::service::build(&edges, from, None)?;
    edge_index.sorted = edge_order;
    let root_order = crate::domain::partition::service::ordered_positions(&roots, &root_columns)?;
    let mut root_index = crate::domain::equality::service::build(&roots, &root_columns, None)?;
    root_index.sorted = root_order;
    Ok(PreparedDfs {
        roots,
        edges,
        root_index,
        edge_index,
        root_columns,
        edge_columns,
        adjacency,
    })
}
