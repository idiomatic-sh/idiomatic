//! The `probe` item of the `kernels` collection: each left row, in order, probes a `hash_index` of the right rows by its key.
//! An inner probe answers the left row with each right row of its key, ascending; a semi probe keeps the left row when its key
//! has a right row, an anti probe when it has none.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::distinct::item::counted;
use super::super::filter::item::{filter, keep};
use super::super::hash_index::item::{cells, find_cells, indexed, next_of};
use super::super::scan_facts::item::{cell_of, count_of, kinds_of, row_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::{KeyedRows, NO_ROW, NodeDecisions};
#[cfg(test)]
use crate::domain::physical_plan::types::Lowering;
use crate::domain::equality_join::types::{EmptyRight, Existence, ProbeKey, ProbeMode, ProbeParts};
use crate::domain::physical_plan::types::{CellStart, InputRows, KernelRun, KeyedRowAt, KeyedSide};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Side};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `probe`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ProbeItem;

#[cfg(test)]
impl KernelsItem for ProbeItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = {
            matches!(lowering, Lowering::Probe { .. })
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// The scratch a probe writes through.
pub(crate) struct ProbeScratch<'s> {
    /// The worker's one keyed-row table.
    pub(crate) index: &'s mut KeyedRows,
    /// The row read, decoded from the facts.
    pub(crate) row: &'s mut Vec<Value>,
    /// The row a filter writes.
    pub(crate) written: &'s mut Vec<Value>,
    /// The places of the answers kept, when repeats are dropped.
    pub(crate) seen: &'s mut hashbrown::HashTable<RowCount>,
    /// The right filter's decisions.
    pub(crate) decisions: &'s mut NodeDecisions,
}

/// The kinds of a probe's answer.
///
/// # Errors
///
/// `NoSuchColumn` for an answer column past its side's rows.
fn answer_kinds(
    probe: &ProbeParts,
    left: InputRows<'_>,
    right: InputRows<'_>,
) -> Result<Vec<ryo_relations::domain::relations::types::CellKind>, PortableOperatorFault> {
    if probe.mode != ProbeMode::Inner {
        return Ok(kinds_of(left).to_vec());
    }
    probe
        .output
        .iter()
        .map(|column| {
            let side = match column.side {
                Side::Left => left,
                Side::Right => right,
            };
            kinds_of(side)
                .get(usize::try_from(column.column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn {
                    index: column.column,
                })
        })
        .collect()
}

/// Appends the answer of each row of `left` probing the rows of `right` to `output`, and answers the first row's fault: the
/// right filter's, before any answer, or a left row's, after the answers of the rows before it.
///
/// # Errors
///
/// `NoSuchColumn` for an answer column, or a right key column, past its rows.
pub(crate) fn probed(
    probe: &ProbeParts,
    left: InputRows<'_>,
    right: InputRows<'_>,
    run: &mut KernelRun<'_>,
    scratch: ProbeScratch<'_>,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    output.kinds = answer_kinds(probe, left, right)?;
    let filtered;
    let right = if probe.filter.comparisons.is_empty() {
        right
    } else {
        let kinds = kinds_of(right);
        let kernel = filter(&probe.filter, kinds, run.resources.texts(), scratch.decisions);
        let mut kept = Table {
            kinds: kinds.to_vec(),
            ..Table::default()
        };
        for at in 0..count_of(right).0 {
            let row = row_of(right, RowIndex(at), scratch.row);
            if let Err(fault) = keep(kernel, run.matcher, row, scratch.decisions, scratch.written, &mut kept) {
                return Ok(Some(fault));
            }
        }
        filtered = kept;
        InputRows::Table(&filtered)
    };
    let (every_left, every_right);
    let (left_on, right_on): (&[ColumnIndex], &[ColumnIndex]) = match &probe.key {
        ProbeKey::Columns { left, right } => (left, right),
        ProbeKey::Whole => {
            every_left = every(left);
            every_right = every(right);
            (&every_left, &every_right)
        }
    };
    let indexed_side = KeyedSide {
        rows: right,
        on: right_on,
    };
    cells(indexed_side, scratch.index)?;
    let empty = count_of(right).0 == 0;
    scratch.seen.clear();
    for at in 0..count_of(left).0 {
        let at = RowIndex(at);
        if probe.mode == ProbeMode::Anti && probe.empty_right == EmptyRight::EveryLeftRow && empty {
            let start = CellStart(output.cells.len());
            output.cells.extend_from_slice(row_of(left, at, scratch.row));
            counted(output, start, probe.repeated, scratch.seen);
            continue;
        }
        let found = match find_cells(
            scratch.index,
            indexed_side,
            KeyedRowAt {
                side: KeyedSide {
                    rows: left,
                    on: left_on,
                },
                row: at,
            },
        ) {
            Ok(found) => found,
            Err(fault) => return Ok(Some(fault)),
        };
        if probe.mode == ProbeMode::Inner {
            let mut matched = found.map_or(NO_ROW, |key| key.first);
            while matched != NO_ROW {
                let start = CellStart(output.cells.len());
                for column in &probe.output {
                    let cell = match column.side {
                        Side::Left => cell_of(left, at, column.column),
                        Side::Right => cell_of(right, indexed(matched), column.column),
                    };
                    let Some(cell) = cell else {
                        output.cells.truncate(start.0);
                        return Ok(Some(PortableOperatorFault::NoSuchColumn {
                            index: column.column,
                        }));
                    };
                    output.cells.push(cell);
                }
                counted(output, start, probe.repeated, scratch.seen);
                matched = next_of(scratch.index, matched);
            }
            continue;
        }
        let exists = match (probe.existence, found) {
            (_, None) => false,
            (Existence::First, Some(_)) => true,
            (Existence::Every, Some(key)) => {
                let mut rows = RowCount(0);
                let mut matched = key.first;
                while matched != NO_ROW {
                    rows = RowCount(rows.0.saturating_add(1));
                    matched = next_of(scratch.index, matched);
                }
                rows.0 > 0
            }
        };
        if exists == (probe.mode == ProbeMode::Semi) {
            let start = CellStart(output.cells.len());
            output.cells.extend_from_slice(row_of(left, at, scratch.row));
            counted(output, start, probe.repeated, scratch.seen);
        }
    }
    Ok(None)
}

/// Every column of the rows, in order.
fn every(rows: InputRows<'_>) -> Vec<ColumnIndex> {
    (0..kinds_of(rows).len())
        .map(|at| ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX)))
        .collect()
}
