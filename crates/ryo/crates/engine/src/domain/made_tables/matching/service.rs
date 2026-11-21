//! The services of the `matching` part: the `Ordered(Match)` fill. Each input is gathered through its pipelines, then
//! indexed by its keys with each key's rows ascending by its order; for each key of the left input the right input's rows
//! of the same key are found, and the operators crate's pairing says which left row takes which right row. A row of the
//! table is the left row then the right row, each whole.

use super::types::{MatchRoom, MatchSide};
use crate::domain::code::types::IndexSite;
use crate::domain::filled::errors::FilledError;
use crate::domain::indexes::reading::service::{cell_at, read, rows_of, run_of, runs};
use crate::domain::indexes::types::{IndexRead, SourceRow};
use crate::domain::made_tables::service::{column, row, rows_indexed};
use crate::domain::made_tables::types::TableRows;
use ryo_operators::domain::ordered_groups::types::GroupOrders;
use ryo_operators::domain::ordered_match::service::{paired, unrepeated};
use ryo_relations::domain::relations::types::{Count, Integer};

/// The place a row's number names.
fn place(numbered: SourceRow) -> Count {
    Count(usize::try_from(numbered.0).unwrap_or(usize::MAX))
}

/// The key columns of a side, as places.
fn keys_of<'c>(side: MatchSide<'c>) -> impl ExactSizeIterator<Item = Count> + 'c {
    side.keys
        .iter()
        .map(|column| Count(usize::try_from(column.0).unwrap_or(usize::MAX)))
}

/// One key's orders on one side, ascending, into `orders`; a fault when two rows of the key hold one order.
fn orders_of(
    input: &TableRows,
    order: Count,
    rows: &[SourceRow],
    orders: &mut Vec<Integer>,
) -> Result<(), FilledError> {
    let ordered = column(input, order);
    orders.clear();
    orders.extend(
        rows.iter()
            .map(|each| Integer(cell_at(ordered, place(*each)).0.cast_signed())),
    );
    unrepeated(GroupOrders(orders)).map_err(|fault| FilledError::Meaning { fault })
}

/// Every key of an index holds no order twice.
fn all_unrepeated(
    input: &TableRows,
    order: Count,
    index: IndexRead<'_>,
    orders: &mut Vec<Integer>,
) -> Result<(), FilledError> {
    for run in runs(index) {
        orders_of(input, order, rows_of(index, run), orders)?;
    }
    Ok(())
}

/// The `Ordered(Match)` fill: for each key the two inputs share, each left row, ascending, with the earliest right row
/// of greater order that no earlier left row took; the left row's cells, then the right row's.
///
/// # Errors
///
/// A key of one input holds one order twice: today's fault of a duplicate order.
pub(crate) fn match_rows(
    left: &TableRows,
    right: &TableRows,
    left_side: MatchSide<'_>,
    right_side: MatchSide<'_>,
    room: &mut MatchRoom,
    out: &mut TableRows,
) -> Result<(), FilledError> {
    rows_indexed(
        left,
        keys_of(left_side),
        Some(left_side.order),
        &mut room.left,
    );
    rows_indexed(
        right,
        keys_of(right_side),
        Some(right_side.order),
        &mut room.right,
    );
    let MatchRoom {
        left: left_index,
        right: right_index,
        left_orders,
        right_orders,
        pairs,
        key,
    } = room;
    let (left_index, right_index) = (
        read(left_index, IndexSite(0)),
        read(right_index, IndexSite(0)),
    );
    all_unrepeated(right, right_side.order, right_index, right_orders)?;
    for run in runs(left_index) {
        let left_rows = rows_of(left_index, run);
        orders_of(left, left_side.order, left_rows, left_orders)?;
        let Some(first) = left_rows.first() else {
            continue;
        };
        key.clear();
        key.extend(keys_of(left_side).map(|at| cell_at(column(left, at), place(*first))));
        let right_rows = rows_of(right_index, run_of(right_index, key));
        orders_of(right, right_side.order, right_rows, right_orders)?;
        pairs.clear();
        paired(GroupOrders(left_orders), GroupOrders(right_orders), pairs);
        for pair in pairs.iter() {
            let taken = (
                left_rows.get(usize::try_from(pair.left.0).unwrap_or(usize::MAX)),
                right_rows.get(usize::try_from(pair.right.0).unwrap_or(usize::MAX)),
            );
            if let (Some(left_at), Some(right_at)) = taken {
                out.cells.extend_from_slice(row(left, place(*left_at)));
                out.cells.extend_from_slice(row(right, place(*right_at)));
                out.count = Count(out.count.0.saturating_add(1));
            }
        }
    }
    Ok(())
}
