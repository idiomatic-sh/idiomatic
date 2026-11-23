//! Evaluate a distinct node count grouped by its fixed ancestor in this file.
use super::{Arena, Bits, Map, NodeTable, Nodes, Position, each, node_table};
use ryo_values::domain::finish_plans::expressions::types::Comparator;

pub(super) fn compare(left: i64, op: Comparator, right: i64) -> bool {
    use Comparator::*;
    // The executor reserves this bit pattern for a null numeric cell.
    if left as u64 == super::super::finish::NULL || right as u64 == super::super::finish::NULL {
        return false;
    }
    match op {
        Equal => left == right,
        NotEqual => left != right,
        Less => left < right,
        LessOrEqual => left <= right,
        Greater => left > right,
        GreaterOrEqual => left >= right,
    }
}

pub(super) fn under(
    table: &NodeTable,
    arena: &mut Arena,
    children: Nodes,
    all: bool,
    depth: u8,
    op: Comparator,
    value: i64,
) -> Nodes {
    let mut counts = Map::<u32, u32>::default();
    let mut count = |mut id: u32| {
        for _ in 0..depth {
            let parent = node_table::parent_of(table, Position(id)).0;
            if parent == id {
                return;
            }
            id = parent;
        }
        *counts.entry(id).or_default() += 1;
    };
    if all {
        (0..node_table::element_count(table).0 as u32).for_each(count);
    } else {
        let children: Bits = children?;
        each(&arena.words[children.range()], |i| {
            count(children.first() as u32 * 64 + i)
        });
    }
    Some(
        arena.bits(
            counts
                .into_iter()
                .filter_map(|(id, n)| compare(i64::from(n), op, value).then_some(id)),
        ),
    )
}
