//! Axis traversal over an immutable borrowed edge population and engine-owned index.
use crate::domain::{
    operator::contract::views::IndexedRows, table_model::types::SetMembership,
    table_reach::types::Walk,
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub fn follow(
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

pub fn reach_breadth(
    edges: &IndexedRows<'_>,
    to: ColumnIndex,
    start: Value,
    walk: &mut Walk,
) {
    match breadth::<_, true>(edges, to, start, walk, |_| {
        Ok::<_, std::convert::Infallible>(SetMembership::Out)
    }) {
        Ok(()) => {}
        Err(never) => match never {},
    }
}

/// Keep only the first admitted vertex in the exact ordinary breadth-first order.
/// The origin is not pre-visited: an edge cycle may reach it just as in `reach_breadth`.
pub(crate) fn reach_nearest<E>(
    edges: &IndexedRows<'_>,
    to: ColumnIndex,
    start: Value,
    walk: &mut Walk,
    admit: impl FnMut(Value) -> Result<SetMembership, E>,
) -> Result<(), E> {
    breadth::<_, false>(edges, to, start, walk, admit)
}

fn breadth<E, const ALL: bool>(
    edges: &IndexedRows<'_>,
    to: ColumnIndex,
    start: Value,
    walk: &mut Walk,
    mut admit: impl FnMut(Value) -> Result<SetMembership, E>,
) -> Result<(), E> {
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
                if !ALL && admit(*entity)? == SetMembership::In {
                    reached.push(*entity);
                    return Ok(());
                }
                follow(edges, to, *entity, key, next);
                if ALL {
                    reached.push(*entity);
                }
            }
        }
        std::mem::swap(frontier, next);
    }
    Ok(())
}

#[cfg(test)]
mod borrowed_test;
