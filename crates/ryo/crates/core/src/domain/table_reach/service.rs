//! The `table_reach` service: following edges from an entity to the ones it leads to, with no IO and no ports.

use ryo_language::domain::relations::types::Value;

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

/// Every entity reachable from `start` by one or more edges, nearest first, each once, into
/// `walk.reached`.
pub(crate) fn reach_breadth(edges: &Edges<'_>, start: Value, walk: &mut Walk) {
    let Walk {
        key,
        frontier,
        next,
        reached,
        visited,
    } = walk;
    visited.clear();
    reached.clear();
    frontier.clear();
    follow(edges, start, key, frontier);
    while !frontier.is_empty() {
        next.clear();
        for entity in frontier.iter() {
            if visited.insert(*entity) {
                follow(edges, *entity, key, next);
                reached.push(*entity);
            }
        }
        std::mem::swap(frontier, next);
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
