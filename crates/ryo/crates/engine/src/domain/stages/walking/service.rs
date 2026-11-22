//! The services of the `walking` part: the `Walk` stage. For each row of the batch, the elements a walk from its start
//! reaches: its children, of one field when one is named, or the elements along a tree axis. Under the file's own tree
//! an embedded element is never reached.

use crate::domain::stages::nodes::service::position_of;
use crate::domain::stages::service::{fanned_of, read_all};
use crate::domain::stages::types::{Batches, FanRows};
use crate::domain::steps::types::AxisDirection;
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::service::{
    children_of, embedded_segments, field_of, own_end, parent_of, subtree_end_of,
};
use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
use ryo_relations::domain::relations::types::{Count, NodeExtent, Position};

/// The field code that names no field: a walk of every child.
const NO_FIELD: FieldCode = FieldCode(0);

/// The id past the last element a walk of the extent may reach.
#[inline]
fn reach(nodes: &NodeTable, extent: NodeExtent) -> Position {
    match extent {
        NodeExtent::Own => own_end(nodes),
        NodeExtent::Embedded => Position(u32::MAX),
    }
}

/// The start of the row the cursor is at.
#[inline]
fn start_at(starts: &[CellWord], fan: &FanRows) -> Position {
    starts
        .get(fan.cursor.row.0)
        .map_or(Position(u32::MAX), |cell| position_of(*cell))
}

/// An element as the number a handle holds of it.
#[inline]
fn numbered(element: Position) -> CellWord {
    CellWord(u64::from(element.0))
}

/// Gives the rows of the cursor's row a walk found, counted first: a walk's rows are not a slice.
#[inline]
fn walked(found: impl Iterator<Item = Position> + Clone, fan: &mut FanRows) -> Batches {
    fanned_of(Count(found.clone().count()), found.map(numbered), fan)
}

/// `Walk` over children: for each row of the batch, from the cursor on, the children of its start, those of `field` alone
/// when it names one. Stops when the batch out is full; answers whether every row in was read.
pub(crate) fn children(
    nodes: &NodeTable,
    extent: NodeExtent,
    field: FieldCode,
    starts: &[CellWord],
    rows: Count,
    fan: &mut FanRows,
) -> Batches {
    let end = reach(nodes, extent);
    while fan.cursor.row.0 < rows.0 {
        let found = children_of(nodes, start_at(starts, fan))
            .iter()
            .copied()
            .filter(|child| child.0 < end.0)
            .filter(|child| field == NO_FIELD || field_of(nodes, *child) == field);
        if walked(found, fan) == Batches::More {
            break;
        }
    }
    read_all(fan, rows)
}

/// The parent of an element the walk may reach: none of the root.
#[inline]
fn up(nodes: &NodeTable, end: Position, from: Position) -> Option<Position> {
    let parent = parent_of(nodes, from);
    (from.0 != 0 && parent != from && parent.0 < end.0).then_some(parent)
}

/// The sibling of an element `ahead` places after it among its parent's children, or with `ahead` nought the one before.
#[inline]
fn sibling(nodes: &NodeTable, end: Position, at: Position, ahead: Count) -> Option<Position> {
    let all = children_of(nodes, up(nodes, end, at)?);
    let place = all.iter().position(|each| *each == at)?;
    let wanted = if ahead.0 == 0 {
        place.checked_sub(1)?
    } else {
        place.checked_add(ahead.0)?
    };
    all.get(wanted).copied().filter(|found| found.0 < end.0)
}

/// SPIKE (ryo-fmx3.1): adds to the descendant nodes `found` of `at` the tokens under each of them and under `at`: a node's
/// tokens stand after the nodes of its tree (the file's own, or an embedded segment's), outside the ids of its subtree, so
/// the walk down by ids reaches none, though they are children of its nodes as a tree relation states them.
fn with_tokens(nodes: &NodeTable, end: Position, at: Position, found: &mut Vec<Position>) {
    let roots: Vec<Position> = embedded_segments(nodes).map(|segment| segment.root).collect();
    let count = found.len();
    for place in 0..=count {
        let Some(node) = (if place == count { Some(at) } else { found.get(place).copied() }) else {
            continue;
        };
        let last = subtree_end_of(nodes, node);
        for child in children_of(nodes, node) {
            let token = (child.0 <= node.0 || child.0 >= last.0) && child.0 < end.0;
            if token && !roots.contains(child) {
                found.push(*child);
            }
        }
    }
}

/// `Walk` along an axis: for each row of the batch, from the cursor on, the parent of its start, each ancestor nearest
/// first, each descendant node in pre-order and then their tokens, or the next or the previous sibling. Stops when the
/// batch out is full; answers whether every row in was read.
pub(crate) fn along(
    nodes: &NodeTable,
    extent: NodeExtent,
    direction: AxisDirection,
    starts: &[CellWord],
    rows: Count,
    fan: &mut FanRows,
) -> Batches {
    let end = reach(nodes, extent);
    while fan.cursor.row.0 < rows.0 {
        let at = start_at(starts, fan);
        let more = match direction {
            AxisDirection::Parent => walked(up(nodes, end, at).into_iter(), fan),
            AxisDirection::Ancestor => walked(
                std::iter::successors(up(nodes, end, at), |each| up(nodes, end, *each)),
                fan,
            ),
            AxisDirection::Descendant => {
                let below = at.0.saturating_add(1)..subtree_end_of(nodes, at).0.min(end.0);
                if extent == NodeExtent::Own || embedded_segments(nodes).next().is_none() {
                    let mut found: Vec<Position> = below.clone().map(Position).collect();
                    with_tokens(nodes, end, at, &mut found);
                    walked(found.into_iter(), fan)
                } else {
                    // SPIKE (ryo-fmx3.1, fault I): an embedded segment stands after the file's own elements, outside the
                    // ids of its host's subtree, and its root's parent is the host: the nodes of each segment hosted
                    // under the start are descendants too, as the walk up from them reaches the start.
                    let mut found: Vec<Position> = below.map(Position).collect();
                    let mut held = vec![(at.0, subtree_end_of(nodes, at).0)];
                    for segment in embedded_segments(nodes) {
                        let hosted = held
                            .iter()
                            .any(|(from, to)| *from <= segment.host.0 && segment.host.0 < *to);
                        if hosted && segment.root.0 > at.0 {
                            let last = subtree_end_of(nodes, segment.root).0;
                            found.extend((segment.root.0..last).map(Position));
                            held.push((segment.root.0, last));
                        }
                    }
                    with_tokens(nodes, end, at, &mut found);
                    walked(found.into_iter(), fan)
                }
            }
            AxisDirection::NextSibling => {
                walked(sibling(nodes, end, at, Count(1)).into_iter(), fan)
            }
            AxisDirection::PreviousSibling => {
                walked(sibling(nodes, end, at, Count(0)).into_iter(), fan)
            }
        };
        if more == Batches::More {
            break;
        }
    }
    read_all(fan, rows)
}
