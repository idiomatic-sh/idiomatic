//! The `closure` operator: every pair of entities joined by a chain of edges.

use ryo_language::domain::relations::types::Value;

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::types::{Emit, Feed, OperatorContext, Room};
use crate::domain::replay::service::replay_computed;
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{
    check_columns, column_in, index, kinds_of, push_row, reset, row_in, table_in,
};
use crate::domain::table_model::types::{On, Rows, Table};
use crate::domain::table_reach::service::reach_all;
use crate::domain::table_reach::types::Edges;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::domain::plans::types::Operation;

/// `closure`, one item of the `operators` collection.
pub(crate) struct ClosureItem;

impl OperatorsItem for ClosureItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Closure {
            source,
            destination,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let edges = first(inputs)?;
        let OperatorScratch {
            row,
            order,
            right,
            walk,
            ..
        } = &mut *context.scratch;
        let right = table_in(edges, right, row);
        check_columns(&right.kinds, &[*source, *destination])?;
        let starts = [*source];
        index(right, On::Columns(&starts), order)?;
        reset(out, &[]);
        for column in [source, destination] {
            out.kinds.push(
                *column_in(kinds_of(edges), *column)
                    .ok_or(TableModelError::NoSuchColumn { index: column.0 })?,
            );
        }
        let walked = Edges {
            table: right,
            order,
            from: &starts,
            to: *destination,
        };
        let mut previous: Option<Value> = None;
        for hit in &walked.order.rows {
            let Some(start) = row_in(right, *hit)
                .and_then(|edge| column_in(edge, *source))
                .copied()
            else {
                continue;
            };
            if previous == Some(start) {
                continue;
            }
            previous = Some(start);
            reach_all(&walked, start, walk);
            for reached in &walk.reached {
                push_row(out, &[start, *reached]);
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
        replay_computed(
            operation,
            ins,
            context,
            room,
            emit,
            |operation, inputs, context, out| ClosureItem.compute(operation, inputs, context, out),
        )
    }
}
