//! The `positions` service: free functions named by verb that read the rows of a lowered step and key them, with no IO and no ports.

use ryo_language::domain::elements::service::{field_name_at, parent_at, text_at};
use ryo_language::domain::relations::service::node_id;
use ryo_language::domain::relations::service::to_bits;
use ryo_language::domain::relations::types::{Flag, Position, Value};

use super::constants::{COLUMN_OF_STEP, POSITION_OF_TREE, ROW_OF_POSITIONS, SLOT_OF_ROW};
use super::types::{
    Keeps, KeyOrder, Keys, LoweredRoom, OrderRun, Positions, RowPositions, Slots, Stride,
};
use crate::domain::lowering::types::{ElementPart, LoweredColumn};
use crate::domain::table_cells::service::{kind_name_at, ordinal_number_at, span_at};
use crate::domain::table_cells::types::{RowIndex, TreeRows};
use crate::domain::table_model::types::RowCount;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::StepId;

/// The value a field holds of the element at `at`.
#[must_use]
pub(crate) fn value(tree: &TreeRows<'_>, field: ElementPart, at: Position) -> Value {
    let elements = tree.elements;
    match field {
        ElementPart::Node => Value::Node(node_id(tree.file, at)),
        ElementPart::Kind => Value::Symbol(kind_name_at(elements, at)),
        ElementPart::Text => Value::Symbol(text_at(elements, at)),
        ElementPart::File => Value::File(tree.file),
        ElementPart::Span => Value::Span(span_at(elements, at)),
        ElementPart::Parent => Value::Node(node_id(tree.file, parent_at(elements, at))),
        ElementPart::Ordinal => Value::Number(ordinal_number_at(elements, at)),
        ElementPart::Role => Value::Symbol(field_name_at(elements, at)),
        ElementPart::Absent => Value::Flag(Flag::Lowered),
    }
}

/// The position the column of a row follows: the element it holds the id of.
#[must_use]
pub(crate) fn followed(
    tree: &TreeRows<'_>,
    column: LoweredColumn,
    row: RowPositions<'_>,
) -> Option<Position> {
    let at = slot_of(row, column);
    match column.field {
        ElementPart::Node => Some(at),
        ElementPart::Parent => Some(parent_at(tree.elements, at)),
        ElementPart::Kind
        | ElementPart::Text
        | ElementPart::File
        | ElementPart::Span
        | ElementPart::Ordinal
        | ElementPart::Role
        | ElementPart::Absent => None,
    }
}

/// The rows made of the step `id`; none for a step past the room.
#[must_use]
pub(crate) fn made_of(made: &[Positions], id: StepId) -> Option<&Positions> {
    made.get(usize::try_from(id.0).unwrap_or(usize::MAX))
}

/// How many rows the positions hold.
#[must_use]
pub(crate) fn count(rows: &Positions) -> RowCount {
    RowCount(rows.slots.0.len().checked_div(rows.width.0).unwrap_or(0))
}

/// The row `at` of the positions.
///
/// # Panics
///
/// On a row past the last: a row index comes only from counting these positions, so another is a bug that must crash, not
/// read as an empty row.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "a row index comes only from counting these positions; one past the last is a bug that must crash, not an empty row"
)]
pub(crate) fn row(rows: &Positions, at: RowIndex) -> RowPositions<'_> {
    RowPositions(
        rows.slots
            .0
            .get(at.0 * rows.width.0..(at.0 + 1) * rows.width.0)
            .expect(ROW_OF_POSITIONS),
    )
}

/// The values of a row's columns, into `into`.
pub(crate) fn values(
    tree: &TreeRows<'_>,
    columns: &[LoweredColumn],
    row: RowPositions<'_>,
    into: &mut Vec<Value>,
) {
    into.clear();
    for column in columns {
        into.push(value(tree, column.field, slot_of(row, *column)));
    }
}

/// The position the column `column` of `row` holds.
///
/// # Panics
///
/// When the column's slot is past the row: a lowered column's slot is one of its step's row, so that is a bug.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "a lowered column's slot is one of its step's row; one past it is a bug that must crash, not read the root"
)]
pub(crate) fn slot_of(row: RowPositions<'_>, column: LoweredColumn) -> Position {
    Position(row.0.get(column.slot.0).copied().expect(SLOT_OF_ROW))
}

/// The column at `index` of a step's columns.
///
/// # Panics
///
/// When the index is past the columns: a key names a column of its step, checked when the query was lowered.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "a key names a column of its step, checked when the query was lowered; another is a bug that must crash"
)]
pub(crate) fn column_at(columns: &[LoweredColumn], index: ColumnIndex) -> LoweredColumn {
    columns
        .get(usize::try_from(index.0).unwrap_or(usize::MAX))
        .copied()
        .expect(COLUMN_OF_STEP)
}

/// The numbers the key columns of a row are stored as, appended to `into`.
pub(crate) fn key_into(
    tree: &TreeRows<'_>,
    columns: &[LoweredColumn],
    row: RowPositions<'_>,
    on: &[ColumnIndex],
    into: &mut Keys,
) {
    let into = &mut into.0;
    for each in on {
        let column = column_at(columns, *each);
        into.push(to_bits(value(tree, column.field, slot_of(row, column))).0);
    }
}

/// The rows of `keys` (`stride` numbers each) in the order of their keys, the first of equal keys first.
pub(crate) fn sorted(keys: &Keys, stride: Stride, order: &mut KeyOrder) {
    let keys = &keys.0;
    let order = &mut order.0;
    let stride = stride.0;
    // The key of a row among `keys`, `stride` numbers each; none past the last row.
    let key_of = |row| keys.get(row * stride..(row + 1) * stride);
    order.clear();
    order.extend(0..keys.len().checked_div(stride).unwrap_or(0));
    order.sort_unstable_by(|left, right| key_of(*left).cmp(&key_of(*right)).then(left.cmp(right)));
}

/// The places in `order` of the rows whose key is `probe`.
#[must_use]
pub(crate) fn hits(keys: &Keys, stride: Stride, order: &KeyOrder, probe: &Keys) -> OrderRun {
    let (keys, order, probe) = (&keys.0, &order.0, &probe.0);
    let stride = stride.0;
    // The key of a row among `keys`, `stride` numbers each; none past the last row.
    let key_of = |row| keys.get(row * stride..(row + 1) * stride);
    let probe = Some(probe.as_slice());
    let from = order.partition_point(|each| key_of(*each) < probe);
    let to = from
        + order
            .get(from..)
            .unwrap_or_default()
            .partition_point(|each| key_of(*each) == probe);
    OrderRun(from..to)
}

/// Marks, in `keep`, the first row of each key.
pub(crate) fn firsts(keys: &Keys, stride: Stride, order: &mut KeyOrder, keep: &mut Keeps) {
    sorted(keys, stride, order);
    let keys = &keys.0;
    let (order, keep) = (&order.0, &mut keep.0);
    let stride = stride.0;
    // The key of a row among `keys`, `stride` numbers each; none past the last row.
    let key_of = |row| keys.get(row * stride..(row + 1) * stride);
    keep.clear();
    keep.resize(order.len(), false);
    let mut last: Option<usize> = None;
    for each in order {
        let new = last.is_none_or(|before| key_of(before) != key_of(*each));
        if new {
            if let Some(first) = keep.get_mut(*each) {
                *first = true;
            }
            last = Some(*each);
        }
    }
}

/// `at`, a position a row of this tree holds, checked to be one of the tree's elements.
///
/// # Panics
///
/// When `at` is past the tree's table: a row's positions are made from the tree it is read with, so that is a bug, never a
/// row to leave out.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "a row's positions are made from the tree it is read with; one past its table is a bug that must crash, not a row left out"
)]
pub(crate) fn in_tree(tree: &TreeRows<'_>, at: Position) -> Position {
    let place = usize::try_from(at.0).unwrap_or(usize::MAX);
    let _within: usize = (place < tree.elements.kind.len())
        .then_some(place)
        .expect(POSITION_OF_TREE);
    at
}

/// The elements the class decided for the step `step` of the rule being run selected, when the pass over the file decided its class.
#[must_use]
pub(crate) fn selected(room: &LoweredRoom, step: StepId) -> Option<&Slots> {
    let class = room
        .picked
        .get(usize::try_from(step.0).unwrap_or(usize::MAX))
        .copied()
        .flatten()?;
    room.classes.get(class.0)
}
