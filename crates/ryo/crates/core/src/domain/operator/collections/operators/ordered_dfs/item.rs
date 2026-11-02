//! Ordered, partitioned DFS edge visits using reusable graph and cursor storage.
use super::super::interface::OperatorsItem;
use crate::domain::operating::{
    errors::OperatingError,
    service::{first, second},
    types::{Emit, Feed, OperatorContext, Room},
};
use crate::domain::replay::service::replay_computed;
use crate::domain::table_model::{
    service::{cell, check_columns, column_in, kinds_of, materialize, push_row, reset, row_in},
    types::{On, Rows, Table},
};
use crate::domain::table_reach::{
    service::{begin_ordered, next_ordered, prepare_ordered},
    types::Edges,
};
use crate::domain::table_scratch::types::{OperatorScratch, ScratchText};
use crate::domain::table_texts::{
    service::{name_text, symbol_text},
    types::Texts,
};
use ryo_language::domain::{
    relations::types::{CellKind, Integer, Value},
    symbols::{
        service::intern_derived,
        types::{ByteSlice, Symbols},
    },
    terms::types::NameText,
};
use ryo_values::domain::{plan_parts::types::ColumnIndex, plans::types::Operation};

/// One deterministic graph walk operation.
pub(crate) struct OrderedDfsItem;

/// Append canonical lookup identities while retaining every original output cell.
fn canonical_columns(
    table: &mut Table,
    columns: &mut [ColumnIndex],
    symbols: &mut Symbols,
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
                symbols,
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
                Value::Symbol(intern_derived(
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

impl OperatorsItem for OrderedDfsItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::OrderedDfs {
            root,
            source,
            destination,
            order,
            edge,
            root_keys,
            edge_keys,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let roots = first(inputs)?;
        let edges = second(inputs)?;
        check_columns(kinds_of(roots), &[*root])?;
        check_columns(kinds_of(roots), root_keys)?;
        check_columns(kinds_of(edges), &[*source, *destination, *order, *edge])?;
        check_columns(kinds_of(edges), edge_keys)?;
        let kind = *column_in(kinds_of(roots), *root).ok_or(OperatingError::ShapeMismatch)?;
        if Some(&kind) != column_in(kinds_of(edges), *source)
            || Some(&kind) != column_in(kinds_of(edges), *destination)
            || column_in(kinds_of(edges), *order) != Some(&CellKind::Number)
            || root_keys.len() != edge_keys.len()
            || root_keys.iter().zip(edge_keys).any(|(root_key, edge_key)| {
                column_in(kinds_of(roots), *root_key) != column_in(kinds_of(edges), *edge_key)
            })
        {
            return Err(OperatingError::ShapeMismatch);
        }
        reset(out, &[]);
        for column in root_keys.iter().chain([root]) {
            out.kinds
                .push(*column_in(kinds_of(roots), *column).ok_or(OperatingError::ShapeMismatch)?);
        }
        for column in [source, destination, edge] {
            out.kinds
                .push(*column_in(kinds_of(edges), *column).ok_or(OperatingError::ShapeMismatch)?);
        }
        out.kinds.push(CellKind::Number);
        let scratch = &mut *context.scratch;
        let join = &mut *scratch.join_scratch;
        let walk = &mut *scratch.walk_scratch;
        let capture = &mut *scratch.capture_scratch;
        materialize(roots, &mut join.domain, &mut scratch.row);
        materialize(edges, &mut join.right, &mut scratch.row);
        walk.dfs_graph.root_columns.clear();
        walk.dfs_graph.root_columns.extend_from_slice(root_keys);
        walk.dfs_graph.root_columns.push(*root);
        walk.dfs_graph.edge_columns.clear();
        walk.dfs_graph.edge_columns.extend_from_slice(edge_keys);
        walk.dfs_graph.edge_columns.extend([*source, *destination]);
        canonical_columns(
            &mut join.domain,
            &mut walk.dfs_graph.root_columns,
            context.symbols,
            context.source,
            context.paths,
            context.arenas,
            &mut capture.text,
        )?;
        canonical_columns(
            &mut join.right,
            &mut walk.dfs_graph.edge_columns,
            context.symbols,
            context.source,
            context.paths,
            context.arenas,
            &mut capture.text,
        )?;
        prepare_ordered(&join.right, *order, &mut join.order, &mut walk.dfs_graph)?;
        crate::domain::table_groups::service::sort_rows(
            &join.domain,
            On::Columns(&walk.dfs_graph.root_columns),
            &mut join.domain_order,
        )?;
        walk.dfs_walk.visited.clear();
        walk.dfs_walk.frames.clear();
        walk.dfs_walk.pending = None;
        walk.dfs_walk.ordinal = Some(0);
        walk.dfs_walk.expanded = 0;
        walk.dfs_walk.examined = 0;
        emit_roots(scratch, root_keys, *root, *source, *destination, *edge, out)
    }
    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        replay_computed(
            operation,
            ins,
            context,
            room,
            emit,
            |operation, inputs, context, out| {
                OrderedDfsItem.compute(operation, inputs, context, out)
            },
        )
    }
}

/// Walk each distinct canonical root and emit visits in source edge order.
fn emit_roots(
    scratch: &mut OperatorScratch,
    root_keys: &[ColumnIndex],
    root: ColumnIndex,
    source: ColumnIndex,
    destination: ColumnIndex,
    edge: ColumnIndex,
    out: &mut Table,
) -> Result<(), OperatingError> {
    let join = &mut *scratch.join_scratch;
    let walk = &mut *scratch.walk_scratch;
    let graph = &mut walk.dfs_graph;
    let columns = graph
        .edge_columns
        .get(
            ..graph
                .edge_columns
                .len()
                .checked_sub(1)
                .ok_or(OperatingError::ShapeMismatch)?,
        )
        .ok_or(OperatingError::ShapeMismatch)?;
    let target = *graph
        .edge_columns
        .last()
        .ok_or(OperatingError::ShapeMismatch)?;
    let canonical_root = *graph
        .root_columns
        .last()
        .ok_or(OperatingError::ShapeMismatch)?;
    let walked = Edges {
        table: &join.right,
        order: &join.order,
        from: columns,
        to: target,
    };
    let mut begin = 0;
    while begin < join.domain_order.sorted.len() {
        let root_at = *join
            .domain_order
            .sorted
            .get(begin)
            .ok_or(OperatingError::ShapeMismatch)?;
        let root_row = row_in(&join.domain, root_at).ok_or(OperatingError::ShapeMismatch)?;
        join.key.clear();
        for column in &graph.root_columns {
            join.key.push(cell(root_row, *column)?);
        }
        scratch.other.clear();
        for column in root_keys.iter().copied().chain([root]) {
            scratch.other.push(cell(root_row, column)?);
        }
        let partition = join
            .key
            .get(..root_keys.len())
            .ok_or(OperatingError::ShapeMismatch)?;
        begin_ordered(
            &walked,
            graph,
            partition,
            cell(root_row, canonical_root)?,
            &mut walk.dfs_walk,
        );
        while let Some((at, ordinal)) = next_ordered(&walked, graph, partition, &mut walk.dfs_walk)?
        {
            let edge_row = row_in(&join.right, at).ok_or(OperatingError::ShapeMismatch)?;
            scratch.row.clear();
            scratch.row.extend_from_slice(&scratch.other);
            for column in [source, destination, edge] {
                scratch.row.push(cell(edge_row, column)?);
            }
            scratch.row.push(Value::Number(ordinal));
            push_row(out, &scratch.row);
        }
        graph.expanded += walk.dfs_walk.expanded;
        graph.examined += walk.dfs_walk.examined;
        begin += 1;
        while begin < join.domain_order.sorted.len() {
            let next_at = *join
                .domain_order
                .sorted
                .get(begin)
                .ok_or(OperatingError::ShapeMismatch)?;
            let next = row_in(&join.domain, next_at).ok_or(OperatingError::ShapeMismatch)?;
            if !graph
                .root_columns
                .iter()
                .zip(&join.key)
                .all(|(column, value)| column_in(next, *column) == Some(value))
            {
                break;
            }
            begin += 1;
        }
    }
    Ok(())
}
