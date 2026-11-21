//! The lowering of an ordered match, and its pairing within one key: the meaning the `ordered_match` kernel and the new
//! engine's `Ordered(Match)` fill both call.
use super::types::MatchedPair;
use crate::domain::ordered_groups::types::{GroupOrders, Place};
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_values::domain::plan_parts::types::JoinColumn;

/// The pairing of one key's rows, first in, first out: `left` and `right` are the orders of the key's rows on each
/// side, ascending. Each left row, in order, takes the earliest right row of greater order that no earlier left row
/// took; a left row that finds none takes nothing, and neither do the left rows after it. The pairs are appended to
/// `pairs` in the left rows' order.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_groups::types::{GroupOrders, Place};
/// use ryo_operators::domain::ordered_match::service::paired;
/// use ryo_operators::domain::ordered_match::types::MatchedPair;
/// use ryo_relations::domain::relations::types::Integer;
///
/// let orders = [Integer(1), Integer(2), Integer(3)];
/// let mut pairs = Vec::new();
/// paired(GroupOrders(&orders), GroupOrders(&orders), &mut pairs);
/// assert_eq!(
///     pairs,
///     [MatchedPair { left: Place(0), right: Place(1) }, MatchedPair { left: Place(1), right: Place(2) }]
/// );
/// ```
pub fn paired(left: GroupOrders<'_>, right: GroupOrders<'_>, pairs: &mut Vec<MatchedPair>) {
    let mut next = 0_usize;
    for (at, order) in left.0.iter().enumerate() {
        let rest = right.0.get(next..).unwrap_or_default();
        next = next.saturating_add(rest.partition_point(|taken| taken <= order));
        if next >= right.0.len() {
            break;
        }
        pairs.push(MatchedPair {
            left: Place(u32::try_from(at).unwrap_or(u32::MAX)),
            right: Place(u32::try_from(next).unwrap_or(u32::MAX)),
        });
        next = next.saturating_add(1);
    }
}

/// Whether one key's orders on one side, ascending, hold no order twice: an ordered match pairs by order, and two rows
/// of one key and one order have none. The one check: today's `ordered_match` kernel and the new engine's fill both call it.
///
/// # Errors
///
/// `DuplicateOrder` when two rows of the key hold the same order.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_groups::types::GroupOrders;
/// use ryo_operators::domain::ordered_match::service::unrepeated;
/// use ryo_relations::domain::relations::types::Integer;
///
/// assert!(unrepeated(GroupOrders(&[Integer(1), Integer(2)])).is_ok());
/// assert!(unrepeated(GroupOrders(&[Integer(2), Integer(2)])).is_err());
/// ```
pub fn unrepeated(orders: GroupOrders<'_>) -> Result<(), DerivedFault> {
    if orders.0.windows(2).any(|pair| pair.first() == pair.last()) {
        return Err(DerivedFault::DuplicateOrder);
    }
    Ok(())
}

/// The one lowering of an ordered match: both sides' keys and orders and its output columns, for the `ordered_match` kernel.
pub(crate) fn lower(
    left_on: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    right_on: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    left_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    right_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    output: &[JoinColumn],
) -> super::types::OrderedParts {
    super::types::OrderedParts {
        left_on: left_on.to_vec(),
        right_on: right_on.to_vec(),
        left_order,
        right_order,
        output: output.to_vec(),
        band: None,
    }
}
