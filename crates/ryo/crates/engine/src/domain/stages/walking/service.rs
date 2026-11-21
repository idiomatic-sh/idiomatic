//! The services of the `walking` part: the `Walk` stage. For each row of the batch, the elements a walk from its start
//! reaches: its children, of one field when one is named, or the elements along a tree axis. Under the file's own tree
//! an embedded element is never reached.

use crate::domain::stages::nodes::service::position_of;
use crate::domain::stages::service::{fanned_of, read_all};
use crate::domain::stages::types::{Batches, FanRows};
use crate::domain::steps::types::AxisDirection;
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::service::{
    children_of, field_of, own_end, parent_of, subtree_end_of,
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

/// `Walk` along an axis: for each row of the batch, from the cursor on, the parent of its start, each ancestor nearest
/// first, each descendant node in pre-order, or the next or the previous sibling. Stops when the batch out is full;
/// answers whether every row in was read.
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
                walked(below.map(Position), fan)
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
