//! The `group` item of the `kernels` collection: rows keyed through `hash_index`'s table, one row per group in the order of
//! its first row, formatted by the aggregation's one meaning with how many rows, or distinct values of a column, it holds.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::hash_index::item::{cells, find_cells, indexed, next_of};
use super::super::scan_facts::item::{cell_of, count_of, kinds_of, row_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::{InputRows, KeyedRowAt, KeyedSide};
#[cfg(test)]
use crate::domain::physical_plan::types::Lowering;
use crate::domain::physical_plan::types::file_buffers::value::{KeyedRows, NO_ROW};
use ryo_relations::domain::relations::types::{CellKind, Count, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `group`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct GroupItem;

#[cfg(test)]
impl KernelsItem for GroupItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = {
            matches!(lowering, Lowering::Group { .. })
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// The scratch a grouping writes through: the keyed-row table, the row read, the distinct values and the row written.
pub(crate) struct GroupScratch<'s> {
    /// The worker's one keyed-row table.
    pub(crate) index: &'s mut KeyedRows,
    /// The row read, decoded from the facts.
    pub(crate) row: &'s mut Vec<Value>,
    /// A group's distinct values.
    pub(crate) values: &'s mut Vec<Value>,
    /// The row written.
    pub(crate) written: &'s mut Vec<Value>,
}

/// Appends one row per group of `input` to `output`, keyed by `keys`, in the order of each group's first row: its keys and
/// its count, of rows or of `distinct`'s values.
///
/// # Errors
///
/// `NoSuchColumn` for a key or counted column past the rows.
pub(crate) fn grouped(
    input: InputRows<'_>,
    keys: &[ColumnIndex],
    distinct: Option<ColumnIndex>,
    scratch: GroupScratch<'_>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let kinds = kinds_of(input);
    let mut answer = Vec::with_capacity(keys.len() + 1);
    for column in keys.iter().chain(distinct.as_ref()) {
        let kind = kinds
            .get(usize::try_from(column.0).unwrap_or(usize::MAX))
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?;
        if answer.len() < keys.len() {
            answer.push(kind);
        }
    }
    answer.push(CellKind::Number);
    output.kinds = answer;
    let side = KeyedSide {
        rows: input,
        on: keys,
    };
    cells(side, scratch.index)?;
    for at in 0..count_of(input).0 {
        let Some(run) = find_cells(
            scratch.index,
            side,
            KeyedRowAt {
                side,
                row: RowIndex(at),
            },
        )?
        else {
            continue;
        };
        if indexed(run.first) != RowIndex(at) {
            continue;
        }
        let counted = match distinct {
            None => run.rows.0,
            Some(column) => {
                scratch.values.clear();
                let mut row = run.first;
                while row != NO_ROW {
                    if let Some(value) = cell_of(input, indexed(row), column) {
                        scratch.values.push(value);
                    }
                    row = next_of(scratch.index, row);
                }
                scratch.values.sort_unstable();
                scratch.values.dedup();
                scratch.values.len()
            }
        };
        let first = row_of(input, RowIndex(at), scratch.row);
        crate::domain::aggregate::service::aggregate(
            first,
            keys,
            Count(counted),
            None,
            scratch.written,
        )?;
        output.cells.extend_from_slice(scratch.written);
        output.count = RowCount(output.count.0 + 1);
    }
    Ok(())
}
