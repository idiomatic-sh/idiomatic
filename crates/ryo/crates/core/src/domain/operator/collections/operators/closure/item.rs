//! The `closure` operator: every pair of entities joined by a chain of edges.

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::types::{Emit, Feed, OperatorContext, Room};
use crate::domain::replay::service::replay_computed;
use crate::domain::table_groups::service::sort_rows;
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{
    check_columns, column_in, index, kinds_of, push_row, reset, row_in, table_in,
};
use crate::domain::table_model::types::{On, Rows, Table};
use crate::domain::table_reach::service::reach_all;
use crate::domain::table_reach::types::Edges;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::domain::plan_parts::types::ColumnIndex;
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
            keys,
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
            other,
            key,
            domain,
            domain_order,
            ..
        } = &mut *context.scratch;
        let right = table_in(edges, right, row);
        check_columns(&right.kinds, &[*source, *destination])?;
        check_columns(&right.kinds, keys)?;
        if !keys.is_empty() {
            sort_rows(right, On::Columns(keys), domain_order)?;
            reset(out, &[]);
            for column in keys.iter().chain([source, destination]) {
                out.kinds.push(
                    *column_in(&right.kinds, *column)
                        .ok_or(TableModelError::NoSuchColumn { index: column.0 })?,
                );
            }
            let endpoint_kinds = [
                *column_in(&right.kinds, *source)
                    .ok_or(TableModelError::NoSuchColumn { index: source.0 })?,
                *column_in(&right.kinds, *destination).ok_or(TableModelError::NoSuchColumn {
                    index: destination.0,
                })?,
            ];
            reset(domain, &endpoint_kinds);
            other.clear();
            key.clear();
            let starts = [ColumnIndex(0)];
            let mut begin = 0;
            while begin < domain_order.sorted.len() {
                let first = row_in(right, domain_order.sorted[begin]).unwrap_or_default();
                key.clear();
                key.extend(
                    keys.iter()
                        .filter_map(|column| column_in(first, *column).copied()),
                );
                reset(domain, &endpoint_kinds);
                let mut end = begin;
                while end < domain_order.sorted.len() {
                    let edge = row_in(right, domain_order.sorted[end]).unwrap_or_default();
                    if !keys
                        .iter()
                        .zip(key.iter())
                        .all(|(column, value)| column_in(edge, *column) == Some(value))
                    {
                        break;
                    }
                    push_row(
                        domain,
                        &[
                            *column_in(edge, *source)
                                .ok_or(TableModelError::NoSuchColumn { index: source.0 })?,
                            *column_in(edge, *destination).ok_or(
                                TableModelError::NoSuchColumn {
                                    index: destination.0,
                                },
                            )?,
                        ],
                    );
                    end += 1;
                }
                index(domain, On::Columns(&starts), order)?;
                let walked = Edges {
                    table: domain,
                    order,
                    from: &starts,
                    to: ColumnIndex(1),
                };
                other.clear();
                other.extend(domain.cells.chunks_exact(2).map(|edge| edge[0]));
                other.sort_unstable();
                other.dedup();
                for start in other.iter().copied() {
                    reach_all(&walked, start, walk);
                    for reached in &walk.reached {
                        row.clear();
                        row.extend_from_slice(key);
                        row.extend_from_slice(&[start, *reached]);
                        push_row(out, row);
                    }
                }
                begin = end;
            }
            return Ok(());
        }
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
        other.clear();
        for hit in &walked.order.rows {
            let Some(start) = row_in(right, *hit)
                .and_then(|edge| column_in(edge, *source))
                .copied()
            else {
                continue;
            };
            other.push(start);
        }
        // The index preserves edge order, so one source can occur in several runs.
        other.sort_unstable();
        other.dedup();
        for start in other.iter().copied() {
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
