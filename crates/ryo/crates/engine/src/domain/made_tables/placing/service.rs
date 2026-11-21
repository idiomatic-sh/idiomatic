//! The services of the `placing` part: the `Group(Places)` fill. The input is gathered through its pipelines and indexed
//! by its keys with each key's rows ascending by its order, as a group's end is; a repeated row is one row; the place of
//! each row is the operators crate's meaning (`ordered_groups::service::places`): how many rows of its key have a
//! strictly smaller order, so rows of one order share a place. A row of the table is the input's row, then its place.

use crate::domain::code::types::IndexSite;
use crate::domain::indexes::reading::service::{cell_at, read, rows_of, runs};
use crate::domain::indexes::types::SourceRow;
use crate::domain::made_tables::columns::types::{ColumnIs, RowColumn};
use crate::domain::made_tables::placing::types::PlaceRoom;
use crate::domain::made_tables::service::{column, file_columns, file_of, grouped, place_of, row};
use crate::domain::made_tables::types::{GroupRoom, TableRows};
use ryo_language::domain::facts::types::CellWord;
use ryo_operators::domain::ordered_groups::service::places;
use ryo_operators::domain::ordered_groups::types::GroupOrders;
use ryo_relations::domain::relations::types::{Count, Integer};

/// The place a row's number names.
fn place(numbered: SourceRow) -> Count {
    Count(usize::try_from(numbered.0).unwrap_or(usize::MAX))
}

/// The `Group(Places)` fill: each distinct row of the gathered `input` and, after its columns, its place among its key's
/// rows by the column `order`. The keys are `room.keys`. One ordered index; for each key, its rows with the repeated
/// ones dropped (only a key that holds an order twice is looked at for them), then the meaning over their orders.
pub(crate) fn place_groups(
    input: &TableRows,
    order: Count,
    room: &mut GroupRoom,
    out: &mut TableRows,
) {
    grouped(input, room, Some(order));
    // The table's row is the input's, then the place: a column that is the file's own in the input is the file's here.
    let columns = input.places.len();
    file_columns(out, file_of(input), |at| {
        if at.0 < columns && place_of(input, at) == RowColumn::File {
            ColumnIs::TheFile
        } else {
            ColumnIs::Written
        }
    });
    let GroupRoom {
        index,
        orders,
        placing,
        ..
    } = room;
    let PlaceRoom {
        rows: sorted,
        places: placed,
    } = placing;
    let index = read(index, IndexSite(0));
    let ordered = column(input, order);
    let order_of = |each: SourceRow| Integer(cell_at(ordered, place(each)).0.cast_signed());
    for run in runs(index) {
        let held = rows_of(index, run);
        orders.clear();
        orders.extend(held.iter().map(|each| order_of(*each)));
        // Rows that are one row have one order: a key that holds no order twice holds no row twice, and its rows are
        // read where the index has them. Else its rows of one order are brought together by what they hold and a
        // repeated row is dropped.
        let tied = orders.windows(2).any(|pair| pair.first() == pair.last());
        let distinct: &[SourceRow] = if tied {
            sorted.clear();
            sorted.extend_from_slice(held);
            sorted.sort_unstable_by(|one, other| {
                order_of(*one)
                    .cmp(&order_of(*other))
                    .then_with(|| row(input, place(*one)).cmp(row(input, place(*other))))
            });
            sorted.dedup_by(|one, other| row(input, place(*one)) == row(input, place(*other)));
            orders.clear();
            orders.extend(sorted.iter().map(|each| order_of(*each)));
            sorted
        } else {
            held
        };
        placed.clear();
        places(GroupOrders(orders), placed);
        for (each, at) in distinct.iter().zip(placed.iter()) {
            out.cells.extend_from_slice(row(input, place(*each)));
            out.cells.push(CellWord(u64::from(at.0)));
            out.count = Count(out.count.0.saturating_add(1));
        }
    }
}
