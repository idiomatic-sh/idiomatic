//! Axis traversal over an immutable borrowed edge population and engine-owned index.
use crate::domain::{operator::contract::views::IndexedRows, table_reach::types::Walk};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(crate) fn follow(
    edges: &IndexedRows<'_>,
    to: ColumnIndex,
    start: Value,
    key: &mut Vec<Value>,
    into: &mut Vec<Value>,
) {
    key.clear();
    key.push(start);
    for hit in edges.matching(key) {
        if let Some(end) = edges.rows.cell(hit, to) {
            into.push(end);
        }
    }
}

pub(crate) fn reach_breadth(
    edges: &IndexedRows<'_>,
    to: ColumnIndex,
    start: Value,
    walk: &mut Walk,
) {
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
    follow(edges, to, start, key, frontier);
    while !frontier.is_empty() {
        next.clear();
        for entity in frontier.iter() {
            if visited.insert(*entity) {
                follow(edges, to, *entity, key, next);
                reached.push(*entity);
            }
        }
        std::mem::swap(frontier, next);
    }
}
