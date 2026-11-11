//! The `borrowed` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::types::{IndexedRows, KeyProbe, MatchingRows};
#[cfg(feature = "diagnostic-row-progress")]
use crate::domain::operating::types::{ProgressAmount, ProgressCounter};
use crate::domain::{table_model::types::SetMembership, table_reach::types::Walk};
use ryo_operators::domain::set_rows::types::RightMembership;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

/// Appends the direct destinations of one start through the indexed edges.
pub fn follow(
    edges: &IndexedRows<'_>,
    to: ColumnIndex,
    start: Value,
    key: &mut Vec<Value>,
    into: &mut Vec<Value>,
) {
    key.clear();
    key.push(start);
    for hit in matching(edges, key) {
        if let Some(end) = edges.rows.cell(hit, to) {
            into.push(end);
        }
    }
}

/// Walks every vertex reachable from one start, breadth first.
pub fn reach_breadth(edges: &IndexedRows<'_>, to: ColumnIndex, start: Value, walk: &mut Walk) {
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

pub(super) fn breadth<E, const ALL: bool>(
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

pub(crate) fn contains(indexed: &IndexedRows<'_>, key: &[Value]) -> RightMembership {
    RightMembership::from(matching(indexed, key).next().is_some())
}

pub(crate) fn probe(indexed: &IndexedRows<'_>, key: &[Value]) -> KeyProbe {
    #[cfg(feature = "diagnostic-row-progress")]
    indexed
        .progress
        .observe(ProgressCounter::IndexProbes, ProgressAmount(1));
    let hash = ryo_derived::domain::key::service::hash_key(key.iter().map(Some));
    let bucket = usize::try_from(hash.0).unwrap_or(0) & indexed.index.heads.len().saturating_sub(1);
    KeyProbe {
        hash,
        at: indexed
            .index
            .heads
            .get(bucket)
            .copied()
            .unwrap_or(ryo_derived::domain::key::types::ChainLink(0)),
    }
}

pub(crate) fn next_match(
    indexed: &IndexedRows<'_>,
    key: &[Value],
    probe: &mut KeyProbe,
) -> Option<RowIndex> {
    use ryo_derived::domain::key::types::ChainLink;
    while probe.at.0 != 0 {
        #[cfg(feature = "diagnostic-row-progress")]
        indexed
            .progress
            .observe(ProgressCounter::IndexRowsExamined, ProgressAmount(1));
        let position = usize::try_from(probe.at.0 - 1).unwrap_or(usize::MAX);
        probe.at = indexed
            .index
            .next
            .get(position)
            .copied()
            .unwrap_or(ChainLink(0));
        let row = indexed.index.rows.get(position).copied()?;
        if indexed.index.hashes.get(position) == Some(&probe.hash)
            && indexed
                .columns
                .iter()
                .zip(key)
                .all(|(column, expected)| indexed.rows.cell(row, *column) == Some(*expected))
        {
            #[cfg(feature = "diagnostic-row-progress")]
            indexed
                .progress
                .observe(ProgressCounter::EqualityMatches, ProgressAmount(1));
            return Some(row);
        }
    }
    None
}

pub(crate) fn matching<'a>(indexed: &'a IndexedRows<'_>, key: &'a [Value]) -> MatchingRows<'a> {
    MatchingRows {
        view: indexed,
        key,
        probe: probe(indexed, key),
    }
}
