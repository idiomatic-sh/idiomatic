//! The `table_reach` service: following edges from an entity to the ones it leads to, with no IO and no ports.

use ryo_relations::domain::relations::types::Value;

use super::types::{Edges, Walk};
use crate::domain::table_model::service::{column_in, matching, row_in};
use crate::domain::table_model::types::On;

/// Pushes onto `into` where the edges lead from `start`, in the order the edges came; `key` is the
/// room the lookup is made with.
pub(crate) fn follow(edges: &Edges<'_>, start: Value, key: &mut Vec<Value>, into: &mut Vec<Value>) {
    key.clear();
    key.push(start);
    for hit in matching(edges.table, On::Columns(edges.from), edges.order, key) {
        if let Some(end) = row_in(edges.table, *hit).and_then(|row| column_in(row, edges.to)) {
            into.push(*end);
        }
    }
}

/// Every entity reachable from `start` by one or more edges, in ascending order, each once, into
/// `walk.reached`.
pub(crate) fn reach_all(edges: &Edges<'_>, start: Value, walk: &mut Walk) {
    let Walk {
        key,
        frontier,
        reached,
        visited,
        ..
    } = walk;
    visited.clear();
    reached.clear();
    frontier.clear();
    follow(edges, start, key, frontier);
    while let Some(entity) = frontier.pop() {
        if visited.insert(entity) {
            follow(edges, entity, key, frontier);
        }
    }
    reached.extend(visited.iter().copied());
    reached.sort_unstable();
}

/// Prepare exact adjacency groups once, sorted numerically within each source.
pub(crate) fn prepare_ordered(
    table: &crate::domain::table_model::types::Table,
    ordinal: ryo_values::domain::plan_parts::types::ColumnIndex,
    index: &mut crate::domain::table_model::types::KeyIndex,
    graph: &mut super::types::OrderedGraph,
) -> Result<(), crate::domain::operating::errors::OperatingError> {
    use crate::domain::operating::errors::OperatingError;
    use crate::domain::table_model::service::{cell, positions};
    graph.adjacency.clear();
    graph.adjacency.resize(table.count.0, 0..0);
    graph.expanded = 0;
    graph.examined = 0;
    let columns = graph
        .edge_columns
        .get(..graph.edge_columns.len().saturating_sub(1))
        .unwrap_or_default();
    for at in positions(table.count) {
        if !matches!(
            cell(row_in(table, at).unwrap_or_default(), ordinal)?,
            Value::Number(_)
        ) {
            return Err(OperatingError::ShapeMismatch);
        }
    }
    crate::domain::table_groups::service::sort_rows(table, On::Columns(columns), index)?;
    let mut begin = 0;
    while begin < index.sorted.len() {
        let head_at = *index
            .sorted
            .get(begin)
            .ok_or(OperatingError::ShapeMismatch)?;
        let head = row_in(table, head_at).unwrap_or_default();
        let mut end = begin + 1;
        while end < index.sorted.len() {
            let next_at = *index.sorted.get(end).ok_or(OperatingError::ShapeMismatch)?;
            let row = row_in(table, next_at).unwrap_or_default();
            if !columns
                .iter()
                .all(|column| column_in(head, *column) == column_in(row, *column))
            {
                break;
            }
            end += 1;
        }
        let ordered = index
            .sorted
            .get_mut(begin..end)
            .ok_or(OperatingError::ShapeMismatch)?;
        ordered.sort_unstable_by_key(|at| {
            row_in(table, *at)
                .and_then(|row| column_in(row, ordinal))
                .copied()
        });
        if ordered.windows(2).any(|pair| {
            let [left, right] = pair else {
                return false;
            };
            row_in(table, *left).and_then(|row| column_in(row, ordinal))
                == row_in(table, *right).and_then(|row| column_in(row, ordinal))
        }) {
            return Err(OperatingError::DuplicateDfsOrder);
        }
        for row in ordered.iter() {
            *graph
                .adjacency
                .get_mut(row.0)
                .ok_or(OperatingError::ShapeMismatch)? = begin..end;
        }
        begin = end;
    }
    crate::domain::table_model::service::index(table, On::Columns(columns), index)?;
    Ok(())
}

/// Look up one newly expanded vertex once and retain its adjacency cursor.
fn ordered_frame(
    edges: &Edges<'_>,
    graph: &super::types::OrderedGraph,
    partition: &[Value],
    entity: Value,
    walk: &mut super::types::OrderedWalk,
) -> std::ops::Range<usize> {
    walk.key.clear();
    walk.key.extend_from_slice(partition);
    walk.key.push(entity);
    walk.expanded += 1;
    matching(edges.table, On::Columns(edges.from), edges.order, &walk.key)
        .next()
        .and_then(|at| graph.adjacency.get(at.0))
        .cloned()
        .unwrap_or(0..0)
}

/// Start one independent DFS; the root is already visited before any edge is emitted.
pub(crate) fn begin_ordered(
    edges: &Edges<'_>,
    graph: &super::types::OrderedGraph,
    partition: &[Value],
    root: Value,
    walk: &mut super::types::OrderedWalk,
) {
    walk.visited.clear();
    walk.frames.clear();
    walk.pending = None;
    walk.ordinal = Some(0);
    walk.expanded = 0;
    walk.examined = 0;
    let _root_is_new = walk.visited.insert(root);
    let frame = ordered_frame(edges, graph, partition, root, walk);
    walk.frames.push(frame);
}

/// Yield an edge visit before considering whether to expand its destination.
pub(crate) fn next_ordered(
    edges: &Edges<'_>,
    graph: &super::types::OrderedGraph,
    partition: &[Value],
    walk: &mut super::types::OrderedWalk,
) -> Result<
    Option<(
        crate::domain::table_cells::types::RowIndex,
        ryo_relations::domain::relations::types::Integer,
    )>,
    crate::domain::operating::errors::OperatingError,
> {
    use crate::domain::operating::errors::OperatingError;
    if let Some(destination) = walk.pending.take()
        && walk.visited.insert(destination)
    {
        let frame = ordered_frame(edges, graph, partition, destination, walk);
        walk.frames.push(frame);
    }
    while let Some(frame) = walk.frames.last_mut() {
        if frame.start == frame.end {
            let _finished = walk.frames.pop();
            continue;
        }
        let at = *edges
            .order
            .sorted
            .get(frame.start)
            .ok_or(OperatingError::ShapeMismatch)?;
        frame.start += 1;
        let ordinal = walk.ordinal.ok_or(OperatingError::DfsOrdinalOverflow)?;
        walk.ordinal = ordinal.checked_add(1);
        walk.examined += 1;
        walk.pending = Some(crate::domain::table_model::service::cell(
            row_in(edges.table, at).unwrap_or_default(),
            edges.to,
        )?);
        return Ok(Some((
            at,
            ryo_relations::domain::relations::types::Integer(ordinal),
        )));
    }
    Ok(None)
}
