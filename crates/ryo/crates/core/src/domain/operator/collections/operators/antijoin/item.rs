//! The `antijoin` operator: keep the left rows that have no match on the right.

use ryo_language::domain::relations::types::Value;

use super::super::interface::OperatorsItem;
use super::collections::strategies::interface::StrategiesItem as _;
use super::collections::strategies::set_equality::item::SetEqualityItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::{first, second};
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::operator::collections::operators::antijoin::collections::strategies::types::Absence;
use crate::domain::positions::types::KeyColumns;
use crate::domain::replay::service::scan;
use crate::domain::replay::types::Pipe;
use crate::domain::table_model::service::{
    count_of, index, kinds_of, positions, push_row, read_row, reset, table_in,
};
use crate::domain::table_model::types::{On, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::domain::plans::types::Operation;

/// `antijoin`, one item of the `operators` collection.
pub(crate) struct AntijoinItem;

impl OperatorsItem for AntijoinItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Antijoin {
            left_on, right_on, ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let (left, right) = (first(inputs)?, second(inputs)?);
        let OperatorScratch {
            row, join_scratch, ..
        } = &mut *context.scratch;
        let crate::domain::table_scratch::types::JoinScratch {
            key,
            order,
            right: looked_up,
            ..
        } = &mut **join_scratch;

        reset(out, kinds_of(left));
        // Nothing on the left stays nothing; nothing on the right matches nothing, so the left stays as it is.
        if count_of(left).0 == 0 {
            return Ok(());
        }
        if count_of(right).0 == 0 {
            for position in positions(count_of(left)) {
                if let Some(left_row) = read_row(left, position, row) {
                    push_row(out, left_row);
                }
            }
            return Ok(());
        }
        let looked_up = table_in(right, looked_up, row);
        index(looked_up, On::Columns(right_on), order)?;
        for position in positions(count_of(left)) {
            let Some(left_row) = read_row(left, position, row) else {
                continue;
            };
            if SetEqualityItem.absent(
                looked_up,
                order,
                KeyColumns {
                    left: left_on,
                    right: right_on,
                },
                left_row,
                key,
            )? == Absence::Absent
            {
                push_row(out, left_row);
            }
        }
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
        let Operation::Antijoin {
            left_on, right_on, ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Room { scratch, held, .. } = room;
        let [left_held, right_held, _] = held;
        // The left side comes first: when nothing is on it nothing is kept, and the right side is not made.
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
            right: looked_up,
            ..
        } = &mut **join_scratch;

        let looked_up = table_in(&right, looked_up, row);
        index(looked_up, On::Columns(right_on), order)?;
        let mut buffer = std::mem::take(&mut scratch.other);
        let outcome = scan(
            &left,
            &mut buffer,
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, left_row: &[Value]| {
                if SetEqualityItem.absent(
                    looked_up,
                    order,
                    KeyColumns {
                        left: left_on,
                        right: right_on,
                    },
                    left_row,
                    key,
                )? == Absence::Absent
                {
                    emit.row(each, left_row)?;
                }
                Ok(())
            }),
        );
        scratch.other = buffer;
        outcome
    }
}
