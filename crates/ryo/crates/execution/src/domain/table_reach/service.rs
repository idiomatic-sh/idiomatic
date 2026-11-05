//! The `table_reach` service: following edges from an entity to the ones it leads to, with no IO and no ports.

use ryo_relations::domain::relations::types::Value;

use super::types::{Edges, Walk};
use {
    crate::domain::table_model::service::matching, crate::domain::table_model::service::row_in,
    ryo_values::domain::rows::service::column_in,
};

use ryo_derived::domain::contract::types::On;

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

/// Look up one newly expanded vertex once and retain its adjacency cursor.
fn ordered_frame(
    edges: &Edges<'_>,
    graph: &ryo_derived::domain::contract::types::PreparedDfs,
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
        .and_then(|at| graph.adjacency().get(at.0))
        .cloned()
        .unwrap_or(0..0)
}

/// Start one independent DFS; the root is already visited before any edge is emitted.
pub(crate) fn begin_ordered(
    edges: &Edges<'_>,
    graph: &ryo_derived::domain::contract::types::PreparedDfs,
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
    graph: &ryo_derived::domain::contract::types::PreparedDfs,
    partition: &[Value],
    walk: &mut super::types::OrderedWalk,
) -> Result<
    Option<(
        ryo_values::domain::rows::types::RowIndex,
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
            .sorted()
            .get(frame.start)
            .ok_or(OperatingError::ShapeMismatch)?;
        frame.start += 1;
        let ordinal = walk.ordinal.ok_or(OperatingError::DfsOrdinalOverflow)?;
        walk.ordinal = ordinal.checked_add(1);
        walk.examined += 1;
        walk.pending = Some(ryo_values::domain::rows::service::cell(
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
