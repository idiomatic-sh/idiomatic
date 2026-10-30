//! The `group-count` operator: group rows by their keys and count each group.

use ryo_language::domain::relations::types::CellKind;
use ryo_language::domain::relations::types::Integer;
use ryo_language::domain::relations::types::Value;

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::types::GroupNumber;
use crate::domain::operating::types::GroupValue;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room as StepRoom};
use crate::domain::replay::types::Pipe;
use crate::domain::table_dedup::service::find_or_add;
use crate::domain::table_dedup::types::RowFound;
use crate::domain::table_groups::service::count_groups;
use crate::domain::table_groups::types::CountRoom;
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{cell, column_in, kinds_of, reset, row_in, table_in};
use crate::domain::table_model::types::{Addition, On, RowCount, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::domain::plans::types::Operation;

/// `group_count`, one item of the `operators` collection.
pub(crate) struct GroupCountItem;

impl OperatorsItem for GroupCountItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::GroupCount {
            keys,
            distinct: distinct_column,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let rows = first(inputs)?;
        let OperatorScratch {
            row,
            key,
            order,
            groups,
            right: counted,
            ..
        } = &mut *context.scratch;
        let counted = table_in(rows, counted, row);
        count_groups(
            counted,
            On::Columns(keys),
            *distinct_column,
            &mut CountRoom {
                order,
                groups,
                values: key,
            },
        )?;
        reset(out, &[]);
        for index in keys {
            out.kinds.push(
                *column_in(kinds_of(rows), *index)
                    .ok_or(TableModelError::NoSuchColumn { index: index.0 })?,
            );
        }
        out.kinds.push(CellKind::Number);
        for group in groups.iter() {
            let first = row_in(counted, group.first).unwrap_or_default();
            for index in keys {
                out.cells.extend(column_in(first, *index));
            }
            out.cells.push(Value::Number(Integer(
                i64::try_from(group.counted.0).unwrap_or(i64::MAX),
            )));
            out.count = RowCount(out.count.0 + 1);
        }
        Ok(())
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut StepRoom,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let Operation::GroupCount {
            keys,
            distinct: distinct_column,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let StepRoom {
            scratch,
            out,
            counts,
            pairs,
            kinds,
            ..
        } = room;
        // The groups are found as the rows come, each by its keys, in the order they first appear.
        kinds.clear();
        kinds.extend_from_slice(ins.kinds(InputPlace(0))?);
        reset(out, &[]);
        for index in keys {
            out.kinds.push(
                *column_in(kinds, *index)
                    .ok_or(TableModelError::NoSuchColumn { index: index.0 })?,
            );
        }
        if let Some(missing) = distinct_column
            .filter(|column| usize::try_from(column.0).map_or(true, |at| at >= kinds.len()))
        {
            return Err(TableModelError::NoSuchColumn { index: missing.0 }.into());
        }
        counts.0.clear();
        pairs.clear();
        scratch.seen.clear();
        let mut probe = std::mem::take(&mut scratch.other);
        let seen = &mut scratch.seen;
        let outcome = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|_: &mut OperatorContext<'_>, row: &[Value]| {
                probe.clear();
                for index in keys {
                    probe.push(cell(row, *index)?);
                }
                let RowFound { at: group, added } = find_or_add(out, seen, &probe);
                if added == Addition::Added {
                    counts.0.push(0);
                }
                // A row counts once, or once for each distinct value of the column when one is named.
                let counted = match distinct_column {
                    None => true,
                    Some(column) => pairs.insert(GroupValue {
                        group: GroupNumber(u32::try_from(group.0).unwrap_or(u32::MAX)),
                        value: cell(row, *column)?,
                    }),
                };
                if counted {
                    #[expect(
                        clippy::indexing_slicing,
                        reason = "a group's count is pushed when the group is added, so every group found has one; another is a bug that must crash, not a row left uncounted"
                    )]
                    {
                        counts.0[group.0] += 1;
                    }
                }
                Ok(())
            }),
        );
        scratch.other = probe;
        outcome?;
        let mut answer = std::mem::take(&mut scratch.row);
        for (group, counted) in counts.0.iter().enumerate() {
            answer.clear();
            answer.extend_from_slice(
                row_in(out, crate::domain::table_cells::types::RowIndex(group)).unwrap_or_default(),
            );
            answer.push(Value::Number(Integer(
                i64::try_from(*counted).unwrap_or(i64::MAX),
            )));
            emit.row(context, &answer)?;
        }
        scratch.row = answer;
        Ok(())
    }
}
