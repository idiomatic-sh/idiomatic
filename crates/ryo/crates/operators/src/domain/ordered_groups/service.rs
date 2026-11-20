//! The `ordered_groups` service: the one meaning of `as-of`, `group-min`, `group-max` and `group-ordinal`, each over one key
//! group's orders, ascending. Pure functions: no IO and no engine type. A tie is never a fault: rows of one order answer together.

use super::types::{GroupOrders, GroupRows, Place};
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_orders::types::{GroupEnd, OrderBound};

/// `as-of`: the rows of `group` nearest to `probe` on the side `bound` names. `Before` and `AtOrBefore` answer the rows of the
/// greatest order below the probe (strictly, or at it too); `After` and `AtOrAfter` the rows of the least order above it. Every
/// row tied at that order is in the run; a probe with no row on that side answers an empty run.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_groups::service::nearest;
/// use ryo_operators::domain::ordered_groups::types::{GroupOrders, GroupRows};
/// use ryo_relations::domain::relations::types::Integer;
/// use ryo_values::domain::plan_orders::types::OrderBound;
/// let events = [Integer(2), Integer(5)];
/// assert_eq!(nearest(GroupOrders(&events), Integer(9), OrderBound::Before), GroupRows(1..2));
/// assert!(nearest(GroupOrders(&events), Integer(1), OrderBound::Before).0.is_empty());
/// ```
#[must_use]
pub fn nearest(group: GroupOrders<'_>, probe: Integer, bound: OrderBound) -> GroupRows {
    let (below, from_probe) = group
        .0
        .split_at(group.0.partition_point(|order| *order < probe));
    let (through, above) = group
        .0
        .split_at(group.0.partition_point(|order| *order <= probe));
    match bound {
        OrderBound::Before => last_run(GroupOrders(below)),
        OrderBound::AtOrBefore => last_run(GroupOrders(through)),
        OrderBound::After => first_run(group, GroupOrders(above)),
        OrderBound::AtOrAfter => first_run(group, GroupOrders(from_probe)),
    }
}

/// `group-min` and `group-max`: the rows of `group` at the end `end` names: every row tied for the least order, or for the
/// greatest. A group with no row answers an empty run.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_groups::service::extreme;
/// use ryo_operators::domain::ordered_groups::types::{GroupOrders, GroupRows};
/// use ryo_relations::domain::relations::types::Integer;
/// use ryo_values::domain::plan_orders::types::GroupEnd;
/// let orders = [Integer(1), Integer(4), Integer(4)];
/// assert_eq!(extreme(GroupOrders(&orders), GroupEnd::Greatest), GroupRows(1..3));
/// ```
#[must_use]
pub fn extreme(group: GroupOrders<'_>, end: GroupEnd) -> GroupRows {
    match end {
        GroupEnd::Least => first_run(group, group),
        GroupEnd::Greatest => last_run(group),
    }
}

/// `group-ordinal`: appends to `out` the place of each row of `group`, in the group's order: how many of its rows have a strictly
/// smaller order. The first is 0, and rows of one order share a place. The group's rows are distinct rows: the caller drops
/// repeated rows before it sorts.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_groups::service::places;
/// use ryo_operators::domain::ordered_groups::types::{GroupOrders, Place};
/// use ryo_relations::domain::relations::types::Integer;
/// let orders = [Integer(3), Integer(3), Integer(8)];
/// let mut out = Vec::new();
/// places(GroupOrders(&orders), &mut out);
/// assert_eq!(out, [Place(0), Place(0), Place(2)]);
/// ```
pub fn places(group: GroupOrders<'_>, out: &mut Vec<Place>) {
    let mut earlier: Option<(Integer, Place)> = None;
    for (order, row) in group.0.iter().zip(0_u32..) {
        let place = match earlier {
            Some((tied, shared)) if tied == *order => shared,
            Some(_) | None => Place(row),
        };
        earlier = Some((*order, place));
        out.push(place);
    }
}

/// The rows tied for the greatest order of `head`, the first rows of a group: its last run; empty when it holds no row.
fn last_run(head: GroupOrders<'_>) -> GroupRows {
    let Some(greatest) = head.0.last() else {
        return GroupRows(0..0);
    };
    GroupRows(head.0.partition_point(|order| order < greatest)..head.0.len())
}

/// The rows tied for the least order of `tail`, the last rows of `group`, by their places in the group: the tail's first run;
/// empty when it holds no row.
fn first_run(group: GroupOrders<'_>, tail: GroupOrders<'_>) -> GroupRows {
    let Some(least) = tail.0.first() else {
        return GroupRows(0..0);
    };
    let skipped = group.0.len().saturating_sub(tail.0.len());
    GroupRows(skipped..skipped + tail.0.partition_point(|order| order <= least))
}
