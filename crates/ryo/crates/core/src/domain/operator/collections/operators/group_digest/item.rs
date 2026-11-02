//! The `group-digest` operator: group rows by their keys and digest each group's set of distinct values of a column.

use ryo_language::domain::relations::types::CellKind;
use ryo_language::domain::relations::types::Integer;
use ryo_language::domain::relations::types::Value;

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use std::hash::Hasher as _;

use crate::domain::operating::types::GroupHash;
use crate::domain::operating::types::GroupNumber;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room as StepRoom};
use crate::domain::replay::types::Pipe;
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_dedup::service::find_or_add;
use crate::domain::table_dedup::types::RowFound;
use crate::domain::table_groups::service::{digest_groups, hash_of};
use crate::domain::table_groups::types::CountRoom;
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{cell, column_in, kinds_of, reset, row_in, table_in};
use crate::domain::table_model::types::{RowCount, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::Texts;
use ryo_values::domain::plans::types::Operation;

/// `group_digest`, one item of the `operators` collection.
pub(crate) struct GroupDigestItem;

impl OperatorsItem for GroupDigestItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::GroupDigest { keys, over, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let rows = first(inputs)?;
        let OperatorContext {
            symbols,
            source,
            paths,
            arenas,
            scratch,
            ..
        } = context;
        let OperatorScratch {
            row,
            join_scratch,
            groups_scratch,
            ..
        } = &mut **scratch;
        let crate::domain::table_scratch::types::JoinScratch {
            key,
            order,
            right: grouped,
            ..
        } = &mut **join_scratch;
        let crate::domain::table_scratch::types::GroupsScratch { groups, hashes, .. } =
            &mut **groups_scratch;

        let grouped = table_in(rows, grouped, row);
        let texts = Texts {
            symbols,
            source: *source,
            paths,
            arenas,
        };
        digest_groups(
            grouped,
            keys,
            *over,
            &texts,
            &mut CountRoom {
                order,
                groups,
                values: key,
            },
            hashes,
        )?;
        reset(out, &[]);
        for index in keys {
            out.kinds.push(
                *column_in(kinds_of(rows), *index)
                    .ok_or(TableModelError::NoSuchColumn { index: index.0 })?,
            );
        }
        out.kinds.extend([CellKind::Number, CellKind::Number]);
        for group in groups.iter() {
            let first = row_in(grouped, group.first).unwrap_or_default();
            for index in keys {
                out.cells.extend(column_in(first, *index));
            }
            out.cells.push(Value::Number(Integer(group.digest.0)));
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
        let Operation::GroupDigest { keys, over, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let StepRoom {
            scratch,
            out,
            kinds,
            ..
        } = room;
        let crate::domain::table_scratch::types::GroupsScratch { hashed, .. } =
            &mut *scratch.groups_scratch;
        kinds.clear();
        kinds.extend_from_slice(ins.kinds(InputPlace(0))?);
        if usize::try_from(over.0).map_or(true, |place| place >= kinds.len()) {
            return Err(TableModelError::NoSuchColumn { index: over.0 }.into());
        }
        reset(out, &[]);
        for index in keys {
            out.kinds.push(
                *column_in(kinds, *index)
                    .ok_or(TableModelError::NoSuchColumn { index: index.0 })?,
            );
        }
        hashed.clear();
        scratch.join_scratch.seen.clear();
        let mut probe = std::mem::take(&mut scratch.other);
        let seen = &mut scratch.join_scratch.seen;
        let outcome = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                probe.clear();
                for index in keys {
                    probe.push(cell(row, *index)?);
                }
                let RowFound { at: group, .. } = find_or_add(out, seen, &probe);
                let texts = Texts {
                    symbols: &*each.symbols,
                    source: each.source,
                    paths: each.paths,
                    arenas: each.arenas,
                };
                hashed.push(GroupHash {
                    group: GroupNumber(u32::try_from(group.0).unwrap_or(u32::MAX)),
                    hash: hash_of(cell(row, *over)?, &texts),
                });
                Ok(())
            }),
        );
        scratch.other = probe;
        outcome?;
        // Each group digests the sorted set of the distinct hashes of its values.
        hashed.sort_unstable();
        hashed.dedup();
        let mut answer = std::mem::take(&mut scratch.row);
        let mut at = 0;
        for group in 0..out.count.0 {
            let mut hasher = rustc_hash::FxHasher::default();
            let mut counted = 0_usize;
            while let Some(each) = hashed.get(at)
                && usize::try_from(each.group.0).unwrap_or(usize::MAX) == group
            {
                hasher.write_u64(each.hash.0);
                counted += 1;
                at += 1;
            }
            answer.clear();
            answer.extend_from_slice(row_in(out, RowIndex(group)).unwrap_or_default());
            answer.push(Value::Number(Integer(hasher.finish().cast_signed())));
            answer.push(Value::Number(Integer(
                i64::try_from(counted).unwrap_or(i64::MAX),
            )));
            emit.row(context, &answer)?;
        }
        scratch.row = answer;
        Ok(())
    }
}
