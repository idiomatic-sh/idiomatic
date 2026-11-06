//! Row keys an operation needs ordered before it is evaluated; the owner orders them.

use super::{
    sets,
    types::{PositionInput, PositionWork, RawKeys},
};
use crate::domain::{
    lowering::types::LoweredShape,
    operating::errors::OperatingError,
    operator::contract::types::WorkCounters,
    positions::{
        service::{count, key_into, row},
        types::Keys,
    },
    table_cells::types::TreeRows,
};
use ryo_values::domain::{
    plan_parts::types::{Input, RepeatedRows},
    plans::types::Operation,
    rows::service::positions,
};

/// Keys of the rows an operation orders, or none when it needs no raw order.
/// The keys leave the scratch so the owner can order them while evaluation borrows it.
pub(crate) fn raw_keys(
    operation: &Operation,
    tree: &TreeRows<'_>,
    inputs: &[PositionInput<'_>],
    output: &LoweredShape,
    work: &mut PositionWork,
    counts: &WorkCounters,
) -> Result<Option<RawKeys>, OperatingError> {
    match operation {
        Operation::Project {
            columns,
            distinct: RepeatedRows::Dropped,
            ..
        } => {
            if columns.is_empty() {
                return Err(OperatingError::WrongOperation);
            }
            let source = inputs
                .first()
                .copied()
                .ok_or(OperatingError::MissingInput { position: 1 })?;
            let mut keyed = work_keys(work);
            for at in positions(count(source.rows)) {
                counts.found(0, 1);
                key_into(
                    tree,
                    &source.shape.columns,
                    row(source.rows, at),
                    columns,
                    &mut keyed,
                );
            }
            Ok(Some(RawKeys {
                keys: keyed,
                stride: columns.len().max(1),
            }))
        }
        Operation::Join {
            right: Input::Step(_),
            right_on,
            ..
        } => {
            let right = inputs
                .get(1)
                .copied()
                .ok_or(OperatingError::MissingInput { position: 2 })?;
            let mut keyed = work_keys(work);
            for at in positions(count(right.rows)) {
                key_into(
                    tree,
                    &right.shape.columns,
                    row(right.rows, at),
                    right_on,
                    &mut keyed,
                );
            }
            Ok(Some(RawKeys {
                keys: keyed,
                stride: right_on.len(),
            }))
        }
        Operation::Antijoin { right_on, .. } => {
            let right = inputs
                .get(1)
                .copied()
                .ok_or(OperatingError::MissingInput { position: 2 })?;
            let mut keyed = work_keys(work);
            for at in positions(count(right.rows)) {
                counts.found(1, 1);
                key_into(
                    tree,
                    &right.shape.columns,
                    row(right.rows, at),
                    right_on,
                    &mut keyed,
                );
            }
            Ok(Some(RawKeys {
                keys: keyed,
                stride: right_on.len(),
            }))
        }
        Operation::Union { .. } => {
            work.staged = sets::union_rows(inputs, output, counts)?;
            let mut keyed = work_keys(work);
            sets::union_keys(tree, output, &work.staged, &mut keyed);
            Ok(Some(RawKeys {
                keys: keyed,
                stride: output.columns.len().max(1),
            }))
        }
        _ => Ok(None),
    }
}

/// The scratch's key storage, emptied, for one operation's keys.
fn work_keys(work: &mut PositionWork) -> Keys {
    let mut keys = std::mem::take(&mut work.keys);
    keys.0.clear();
    keys
}
