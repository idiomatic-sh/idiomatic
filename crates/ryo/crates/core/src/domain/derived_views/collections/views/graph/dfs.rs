//! Canonical lookup columns and immutable ordered-DFS adjacency preparation.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{graph::PreparedDfs, views::BorrowedRows};
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_groups::service::sort_rows;
use crate::domain::table_model::{
    service::{check_columns, column_in, push_row},
    types::{KeyIndex, On, Table},
};
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
/// Append canonical lookup identities while retaining every original output cell.
fn canonical_columns(
    table: &mut Table,
    columns: &mut [ColumnIndex],
    symbols: &mut QuerySymbols<'_>,
    texts_source: crate::domain::table_texts::types::SourceView<'_>,
    paths: &[ryo_language::domain::files::types::FilePath],
    arenas: &[ryo_values::domain::stored_facts::types::ArenaBytes],
    buffer: &mut ScratchText,
) -> Result<(), OperatingError> {
    let width = table.kinds.len();
    let after = width
        .checked_add(columns.len())
        .ok_or(OperatingError::ShapeMismatch)?;
    let cells = after
        .checked_mul(table.count.0)
        .ok_or(OperatingError::ShapeMismatch)?;
    let _addressable_width =
        u32::try_from(after).map_err(|_error| OperatingError::ShapeMismatch)?;
    for column in columns.iter() {
        table
            .kinds
            .push(*column_in(&table.kinds, *column).ok_or(OperatingError::ShapeMismatch)?);
    }
    table.cells.resize(cells, Value::Number(Integer(0)));
    for at in (0..table.count.0).rev() {
        table
            .cells
            .copy_within(at * width..(at + 1) * width, at * after);
        for (place, column) in columns.iter().enumerate() {
            let index =
                usize::try_from(column.0).map_err(|_error| OperatingError::ShapeMismatch)?;
            let source_cell = at
                .checked_mul(after)
                .and_then(|row| row.checked_add(index))
                .ok_or(OperatingError::ShapeMismatch)?;
            let value = *table
                .cells
                .get(source_cell)
                .ok_or(OperatingError::ShapeMismatch)?;
            let texts = Texts {
                symbols: symbols.as_ref(),
                source: texts_source,
                paths,
                arenas,
            };
            let found = match value {
                Value::Symbol(symbol) => symbol_text(&texts, symbol),
                Value::Name(name) => {
                    Some(name_text(&texts, name).ok_or(OperatingError::ShapeMismatch)?)
                }
                Value::Node(_)
                | Value::File(_)
                | Value::Span(_)
                | Value::Number(_)
                | Value::Flag(_) => None,
            };
            let canonical = if let Some(text) = found {
                buffer.clear();
                buffer.push_str(text.0);
                Value::Symbol(query_intern_derived(
                    symbols,
                    ByteSlice(texts_source.0),
                    NameText(buffer.as_str()),
                ))
            } else {
                value
            };
            let canonical_cell = at
                .checked_mul(after)
                .and_then(|row| row.checked_add(width))
                .and_then(|row| row.checked_add(place))
                .ok_or(OperatingError::ShapeMismatch)?;
            *table
                .cells
                .get_mut(canonical_cell)
                .ok_or(OperatingError::ShapeMismatch)? = canonical;
        }
    }
    for (place, column) in columns.iter_mut().enumerate() {
        *column = ColumnIndex(
            u32::try_from(width + place).map_err(|_error| OperatingError::ShapeMismatch)?,
        );
    }
    Ok(())
}

/// Canonicalize graph lookup aliases once, retaining untouched original output cells.
pub(super) fn prepare(
    program: &crate::domain::operator::contract::resources::DfsResources,
    roots: BorrowedRows<'_>,
    edges: BorrowedRows<'_>,
    symbols: &mut QuerySymbols<'_>,
    source_view: crate::domain::table_texts::types::SourceView<'_>,
    paths: &[ryo_language::domain::files::types::FilePath],
    arenas: &[ryo_values::domain::stored_facts::types::ArenaBytes],
    buffer: &mut Vec<Value>,
) -> Result<PreparedDfs, OperatingError> {
    let crate::domain::operator::contract::resources::DfsResources {
        root,
        source,
        destination,
        order,
        edge,
        root_keys,
        edge_keys,
    } = program;
    check_columns(roots.kinds(), &[*root])?;
    check_columns(roots.kinds(), root_keys)?;
    check_columns(edges.kinds(), &[*source, *destination, *order, *edge])?;
    check_columns(edges.kinds(), edge_keys)?;
    let kind = *column_in(roots.kinds(), *root).ok_or(OperatingError::ShapeMismatch)?;
    if Some(&kind) != column_in(edges.kinds(), *source)
        || Some(&kind) != column_in(edges.kinds(), *destination)
        || column_in(edges.kinds(), *order) != Some(&CellKind::Number)
        || root_keys.len() != edge_keys.len()
        || root_keys.iter().zip(edge_keys).any(|(root_key, edge_key)| {
            column_in(roots.kinds(), *root_key) != column_in(edges.kinds(), *edge_key)
        })
    {
        return Err(OperatingError::ShapeMismatch);
    }
    let mut prepared = PreparedDfs {
        roots: Table {
            kinds: roots.kinds().to_vec(),
            ..Table::default()
        },
        edges: Table {
            kinds: edges.kinds().to_vec(),
            ..Table::default()
        },
        root_index: KeyIndex::default(),
        edge_index: KeyIndex::default(),
        layout: crate::domain::table_reach::types::OrderedGraph::default(),
    };
    for at in 0..roots.count() {
        if let Some(row) = roots.read(RowIndex(at), buffer) {
            push_row(&mut prepared.roots, row);
        }
    }
    for at in 0..edges.count() {
        if let Some(row) = edges.read(RowIndex(at), buffer) {
            push_row(&mut prepared.edges, row);
        }
    }
    prepared.layout.root_columns.extend_from_slice(root_keys);
    prepared.layout.root_columns.push(*root);
    prepared.layout.edge_columns.extend_from_slice(edge_keys);
    prepared.layout.edge_columns.extend([*source, *destination]);
    let mut text = ScratchText::default();
    canonical_columns(
        &mut prepared.roots,
        &mut prepared.layout.root_columns,
        symbols,
        source_view,
        paths,
        arenas,
        &mut text,
    )?;
    canonical_columns(
        &mut prepared.edges,
        &mut prepared.layout.edge_columns,
        symbols,
        source_view,
        paths,
        arenas,
        &mut text,
    )?;
    crate::domain::table_reach::service::prepare_ordered(
        &prepared.edges,
        *order,
        &mut prepared.edge_index,
        &mut prepared.layout,
    )?;
    sort_rows(
        &prepared.roots,
        On::Columns(&prepared.layout.root_columns),
        &mut prepared.root_index,
    )?;
    Ok(prepared)
}

use crate::domain::table_scratch::types::ScratchText;
use crate::domain::table_texts::service::{name_text, symbol_text};
use ryo_language::domain::relations::types::Integer;
use ryo_language::domain::symbols::{
    service::query_intern_derived,
    types::{ByteSlice, QuerySymbols},
};
use ryo_language::domain::terms::types::NameText;
