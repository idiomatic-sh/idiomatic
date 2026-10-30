//! The `union` operator.

use ryo_language::domain::relations::types::Value;

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::{first, second};
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::replay::types::Pipe;
use crate::domain::table_dedup::service::{distinct, first_time};
use crate::domain::table_model::service::{
    count_of, kinds_of, positions, push_row, read_row, reset,
};
use crate::domain::table_model::types::{Addition, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::domain::plans::types::Operation;

/// `union`, one item of the `operators` collection.
pub(crate) struct UnionItem;

impl OperatorsItem for UnionItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Union { .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let (left, right) = (first(inputs)?, second(inputs)?);
        if kinds_of(left) != kinds_of(right) {
            return Err(OperatingError::ShapeMismatch);
        }
        reset(out, kinds_of(left));
        let OperatorScratch {
            row: buffer, seen, ..
        } = &mut *context.scratch;
        for side in [left, right] {
            for position in positions(count_of(side)) {
                if let Some(row) = read_row(side, position, buffer) {
                    push_row(out, row);
                }
            }
        }
        distinct(out, seen);
        Ok(())
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let Operation::Union { .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        if ins.kinds(InputPlace(0))? != ins.kinds(InputPlace(1))? {
            return Err(OperatingError::ShapeMismatch);
        }
        let Room { scratch, out, .. } = room;
        reset(out, ins.kinds(InputPlace(0))?);
        scratch.seen.clear();
        let seen = &mut scratch.seen;
        // Each row passes once, from whichever side it first comes.
        for which in 0..2 {
            ins.stream(
                InputPlace(which),
                context,
                &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                    if first_time(out, seen, row) == Addition::Added {
                        emit.row(each, row)
                    } else {
                        Ok(())
                    }
                }),
            )?;
        }
        Ok(())
    }
}
