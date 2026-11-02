//! The `difference` operator.

use ryo_language::domain::relations::types::Value;

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::{first, second};
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::replay::service::scan;
use crate::domain::replay::types::Pipe;
use crate::domain::table_dedup::service::{distinct, first_time};
use crate::domain::table_model::service::{
    count_of, index, key_of, kinds_of, matches_empty, matching, positions, push_row, read_row,
    reset, table_in,
};
use crate::domain::table_model::types::{Addition, Emptiness, On, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::domain::plans::types::Operation;

/// `difference`, one item of the `operators` collection.
pub(crate) struct DifferenceItem;

impl OperatorsItem for DifferenceItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Difference { .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let (left, right) = (first(inputs)?, second(inputs)?);
        if kinds_of(left) != kinds_of(right) {
            return Err(OperatingError::ShapeMismatch);
        }
        let OperatorScratch {
            row, join_scratch, ..
        } = &mut *context.scratch;
        let crate::domain::table_scratch::types::JoinScratch {
            key,
            order,
            seen,
            right: looked_up,
            ..
        } = &mut **join_scratch;

        let looked_up = table_in(right, looked_up, row);
        index(looked_up, On::Whole, order)?;
        reset(out, kinds_of(left));
        for position in positions(count_of(left)) {
            let Some(left_row) = read_row(left, position, row) else {
                continue;
            };
            key_of(left_row, On::Whole, key)?;
            let found = matching(looked_up, On::Whole, order, key);
            if matches_empty(&found) == Emptiness::Empty {
                push_row(out, left_row);
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
        let Operation::Difference { .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        if ins.kinds(InputPlace(0))? != ins.kinds(InputPlace(1))? {
            return Err(OperatingError::ShapeMismatch);
        }
        let Room {
            scratch, held, out, ..
        } = room;
        // The left side comes first: when nothing is on it the result is empty, and the right side is not made.
        let [left_held, right_held, _] = held;
        let left = ins.hold(InputPlace(0), context, left_held)?;
        if count_of(&left).0 == 0 {
            return Ok(());
        }
        let right = ins.hold(InputPlace(1), context, right_held)?;
        let OperatorScratch {
            row, join_scratch, ..
        } = scratch;
        let crate::domain::table_scratch::types::JoinScratch {
            key,
            order,
            seen,
            right: looked_up,
            ..
        } = &mut **join_scratch;

        let looked_up = table_in(&right, looked_up, row);
        index(looked_up, On::Whole, order)?;
        reset(out, ins.kinds(InputPlace(0))?);
        seen.clear();
        let mut buffer = std::mem::take(&mut scratch.other);
        let outcome = scan(
            &left,
            &mut buffer,
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, left_row: &[Value]| {
                key_of(left_row, On::Whole, key)?;
                let found =
                    matches_empty(&matching(looked_up, On::Whole, order, key)) == Emptiness::Filled;
                if !found && first_time(out, seen, left_row) == Addition::Added {
                    emit.row(each, left_row)?;
                }
                Ok(())
            }),
        );
        scratch.other = buffer;
        outcome
    }
}
