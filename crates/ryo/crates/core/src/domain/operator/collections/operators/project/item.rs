//! The `project` operator: keep the columns at these places and drop repeated rows.

use ryo_language::domain::relations::types::{CellKind, Value};

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::replay::types::Pipe;
use crate::domain::table_dedup::service::{distinct, first_time};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{
    cell, column_in, count_of, kinds_of, positions, push_row, read_row, reset,
};
use crate::domain::table_model::types::{Addition, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::Operation;

/// `project`, one item of the `operators` collection.
pub(crate) struct ProjectItem;

impl OperatorsItem for ProjectItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Project {
            columns,
            distinct: drops,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let rows = first(inputs)?;
        reset(out, &[]);
        for index in columns {
            out.kinds.push(
                *column_in(kinds_of(rows), *index)
                    .ok_or(TableModelError::NoSuchColumn { index: index.0 })?,
            );
        }
        let OperatorScratch {
            row: buffer,
            other: projected,
            join_scratch,
            ..
        } = &mut *context.scratch;

        for position in positions(count_of(rows)) {
            let Some(row) = read_row(rows, position, buffer) else {
                continue;
            };
            projected.clear();
            for index in columns {
                projected.push(cell(row, *index)?);
            }
            push_row(out, projected);
        }
        if *drops == RepeatedRows::Dropped {
            distinct(out, &mut join_scratch.seen);
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
        let Operation::Project {
            columns,
            distinct: drops,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Room { scratch, out, .. } = room;
        let mut projected = std::mem::take(&mut scratch.other);
        if *drops == RepeatedRows::Dropped {
            // The rows already passed on are kept, to pass each on once.
            reset(out, &[]);
            out.kinds.resize(columns.len(), CellKind::Number);
            scratch.join_scratch.seen.clear();
        }
        let mut seen = (*drops == RepeatedRows::Dropped).then(|| &mut scratch.join_scratch.seen);
        let outcome = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                projected.clear();
                for index in columns {
                    projected.push(cell(row, *index)?);
                }
                if let Some(seen) = seen.as_deref_mut()
                    && first_time(out, seen, &projected) == Addition::Existing
                {
                    return Ok(());
                }
                emit.row(each, &projected)
            }),
        );
        scratch.other = projected;
        outcome
    }
}
