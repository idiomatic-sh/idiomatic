//! Physical anti/union helpers for deferred tree positions. Ordinary Join is private to the fragment capability bridge.
use crate::domain::lowering::{service::shape_at, types::Lowering};
use crate::domain::operating::errors::OperatingError;
use crate::domain::positions::{
    service::{count, firsts, hits, key_into, made_of, row, slot_of, sorted, value},
    types::{KeyColumns, LoweredRoom, Sides, Slots, Stride},
};
use crate::domain::table_cells::types::TreeRows;
use crate::domain::table_model::service::positions;
use ryo_relations::domain::relations::{service::to_bits, types::Position};
use ryo_values::domain::plan_parts::types::StepId;

/// The rows of the left step whose key no row of the right step has.
///
/// # Errors
///
/// `NotRun` when either step has not made its rows.
pub(crate) fn antijoin(
    tree: &TreeRows<'_>,
    lowering: &Lowering,
    sides: Sides,
    keys_of: KeyColumns<'_>,
    room: &mut LoweredRoom,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let out = &mut out.0;
    let left_shape = shape_at(lowering, sides.left)?;
    let right_shape = shape_at(lowering, sides.right)?;
    let LoweredRoom {
        made,
        keys,
        probe,
        order,
        ..
    } = room;
    let left = made_of(made, sides.left).ok_or(OperatingError::NotRun)?;
    let right = made_of(made, sides.right).ok_or(OperatingError::NotRun)?;
    let stride = Stride(keys_of.right.len());
    keys.0.clear();
    for at in positions(count(right)) {
        key_into(
            tree,
            &right_shape.columns,
            row(right, at),
            keys_of.right,
            keys,
        );
    }
    sorted(keys, stride, order);
    for at in positions(count(left)) {
        let each = row(left, at);
        probe.0.clear();
        key_into(tree, &left_shape.columns, each, keys_of.left, probe);
        if hits(keys, stride, order, probe).0.is_empty() {
            out.extend_from_slice(each.0);
        }
    }
    Ok(())
}

/// The rows of both steps, each once.
///
/// # Errors
///
/// `NotRun` when either step, or this one, has not made its shape or rows.
pub(crate) fn union(
    tree: &TreeRows<'_>,
    lowering: &Lowering,
    id: StepId,
    sides: Sides,
    room: &mut LoweredRoom,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let out = &mut out.0;
    let shape = shape_at(lowering, id)?;
    let stride = Stride(shape.columns.len().max(1));
    let LoweredRoom {
        made,
        keys,
        order,
        keep,
        ..
    } = room;
    for side in [sides.left, sides.right] {
        let columns = shape_at(lowering, side)?;
        let rows = made_of(made, side).ok_or(OperatingError::NotRun)?;
        for at in positions(count(rows)) {
            let each = row(rows, at);
            out.extend(
                columns
                    .columns
                    .iter()
                    .map(|column| slot_of(each, *column).0),
            );
        }
    }
    keys.0.clear();
    for each in out.chunks(stride.0) {
        for (position, column) in each.iter().zip(&shape.columns) {
            keys.0
                .push(to_bits(value(tree, column.field, Position(*position))).0);
        }
    }
    firsts(keys, stride, order, keep);
    let mut write = 0;
    for (at, kept) in keep.0.iter().enumerate() {
        if *kept {
            out.copy_within(at * stride.0..(at + 1) * stride.0, write * stride.0);
            write += 1;
        }
    }
    out.truncate(write * stride.0);
    Ok(())
}
