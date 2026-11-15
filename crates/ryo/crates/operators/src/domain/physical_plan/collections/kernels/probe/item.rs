//! The `probe` item of the `kernels` collection: each left row, in order, probes a `hash_index` of the right rows by its key.
//! An inner probe answers the left row with each right row of its key, ascending; a semi probe keeps the left row when its key
//! has a right row, an anti probe when it has none, and an ordered probe when none of them orders strictly past it toward
//! the probe's extremum (each key's extremum found once, as its first row).

use super::super::distinct::item::counted;
use super::super::filter::item::{filter, keep, kept_nodes, readied};
use super::super::hash_index::item::{cells, emptied, find_cells, indexed, next_of};
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{cell_of, count_of, kinds_of, row_of};
use super::super::scan_roots::item::{keyed_roots, next_found};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::{
    EmptyRight, Existence, OrderedProbe, PartitionExtremum, ProbeKey, ProbeMode, ProbeParts,
};
use crate::domain::physical_plan::service::run::types::StagePrepared;
use crate::domain::physical_plan::types::file_buffers::value::{
    KeyedRows, KeyedRun, NO_ROW, NodeDecisions,
};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::{
    CellStart, InputRows, KernelRun, KeyedRowAt, KeyedSide, NodeFed, TreeRead,
};
use ryo_relations::domain::relations::types::{CellKind, NodeId, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Side};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `probe`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ProbeItem;

#[cfg(test)]
impl KernelsItem for ProbeItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = { matches!(lowering, Lowering::Probe { .. }) };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Whether a probe builds the index it reads, reads one a probe of the same right rows and key built in this file, or reads
/// none: the tree it walks is its index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IndexBuild {
    /// It builds the index.
    Builds,
    /// The index is built.
    Built,
    /// No index: each left row's matches are walked from its node.
    Walked,
}

/// The scratch a probe writes through, its own across its left rows: its index, the right rows its filter kept and the key
/// columns of a probe of whole rows, beside the worker's row buffers.
pub(crate) struct ProbeScratch<'s> {
    /// The index it reads: its own keyed-row table, or a shared index.
    pub(crate) index: &'s mut KeyedRows,
    /// Whether it builds that index.
    pub(crate) build: IndexBuild,
    /// The right rows its filter kept, read by every left row; empty when it filters none.
    pub(crate) kept: &'s mut Table,
    /// The key columns of a probe of whole rows, its left's and its right's: every column.
    pub(crate) whole: [&'s mut Vec<ColumnIndex>; 2],
    /// The row read, decoded from the facts.
    pub(crate) row: &'s mut Vec<Value>,
    /// The row a filter writes.
    pub(crate) written: &'s mut Vec<Value>,
    /// The answers kept, keyed by their cells, when repeats are dropped.
    pub(crate) repeats: &'s mut KeyedRows,
    /// The right filter's decisions.
    pub(crate) decisions: &'s mut NodeDecisions,
    /// The tree nodes one left row found, for a probe that walks the tree.
    pub(crate) found: &'s mut Vec<NodeId>,
}

/// The right rows one left row matches: a key's run of the index, or the tree rows its walk found.
#[derive(Clone, Copy, Debug)]
enum Matched<'a> {
    /// The key's rows in the index; none when it has none.
    Indexed(Option<KeyedRun>),
    /// The rows of this tree read the walk found, in the probe's `found`.
    Walked(TreeRead<'a>),
}

/// The kinds of a probe's answer over left rows of `left` and right rows of `right`.
///
/// # Errors
///
/// `NoSuchColumn` for an answer column past its side's rows.
pub(crate) fn answer_kinds(
    probe: &ProbeParts,
    left: &[CellKind],
    right: &[CellKind],
) -> Result<Vec<CellKind>, PortableOperatorFault> {
    if probe.mode != ProbeMode::Inner {
        return Ok(left.to_vec());
    }
    probe
        .output
        .iter()
        .map(|column| {
            let side = match column.side {
                Side::Left => left,
                Side::Right => right,
            };
            side.get(usize::try_from(column.column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn {
                    index: column.column,
                })
        })
        .collect()
}

/// The tree a walked probe reads its right rows from in place; none for a probe of an index.
///
/// # Errors
///
/// `WrongOperation` for a walked probe whose right rows are not the tree's.
fn walked_tree<'a>(
    build: IndexBuild,
    right: InputRows<'a>,
) -> Result<Option<TreeRead<'a>>, PortableOperatorFault> {
    match (build, right) {
        (IndexBuild::Walked, InputRows::Tree(tree)) => Ok(Some(*tree)),
        (IndexBuild::Walked, InputRows::Facts(_) | InputRows::Table(_) | InputRows::Row(..)) => {
            Err(PortableOperatorFault::WrongOperation)
        }
        (IndexBuild::Builds | IndexBuild::Built, _) => Ok(None),
    }
}

/// Whether the probe reads the right rows its filter kept, not `right` as it is: it filters and does not walk the tree.
fn filters(probe: &ProbeParts, build: IndexBuild) -> bool {
    !probe.filter.comparisons.is_empty() && build != IndexBuild::Walked
}

/// The probe's work before it reads its first left row, in its held order: its right filter over `right` into its kept rows,
/// its key columns, its index build and an ordered probe's extremes. A walked probe readies its found-row filter. A probe
/// whose left has no row does none of it.
///
/// # Errors
///
/// `NoSuchColumn` for a right key column past its rows; `ShapeMismatch` for an ordered probe's order that is no number.
pub(crate) fn probe_prepared(
    probe: &ProbeParts,
    left: &[CellKind],
    right: InputRows<'_>,
    run: &KernelRun<'_>,
    scratch: &mut ProbeScratch<'_>,
) -> Result<StagePrepared, PortableOperatorFault> {
    let walked = walked_tree(scratch.build, right)?;
    scratch.kept.kinds.clear();
    scratch.kept.cells.clear();
    scratch.kept.count = RowCount(0);
    if filters(probe, scratch.build) {
        let kinds = kinds_of(right);
        let kernel = filter(&probe.filter, kinds, run.resources.texts());
        readied(&probe.filter, scratch.decisions, kernel.names);
        scratch.kept.kinds.extend_from_slice(kinds);
        for at in 0..count_of(right).0 {
            let read = row_of(right, RowIndex(at), scratch.row);
            if let Err(fault) = keep(
                kernel,
                run.matcher,
                read,
                scratch.decisions,
                scratch.written,
                scratch.kept,
            ) {
                return Ok(StagePrepared { fault: Some(fault) });
            }
        }
    }
    if walked.is_some() && !probe.filter.comparisons.is_empty() {
        let kernel = filter(&probe.filter, kinds_of(right), run.resources.texts());
        readied(&probe.filter, scratch.decisions, kernel.names);
    }
    let right = if filters(probe, scratch.build) {
        InputRows::Table(&*scratch.kept)
    } else {
        right
    };
    let [left_on, right_on] = &mut scratch.whole;
    if matches!(probe.key, ProbeKey::Whole) {
        every(left, left_on);
        every(kinds_of(right), right_on);
    }
    let on = match &probe.key {
        ProbeKey::Columns { right, .. } => right.as_slice(),
        ProbeKey::Whole => right_on.as_slice(),
    };
    if scratch.build == IndexBuild::Builds
        && let Err(fault) = cells(KeyedSide { rows: right, on }, scratch.index)
    {
        // A partial index is no index: the next probe of a shared one builds it again.
        emptied(scratch.index);
        return Err(fault);
    }
    // An ordered probe keeps its own index (`shared_indexes`), which it builds: its keys' first rows become their extremes.
    if let ProbeMode::Ordered(order) = probe.mode {
        extremes(right, order, scratch.index)?;
    }
    Ok(StagePrepared::default())
}

/// Appends the answer of row `at` of `left` probing the right rows to `output`, readied by `probe_prepared` from `right`, and
/// answers the row's fault. A probe that walks the tree finds the row's rows from its node and counts them as the rows of
/// `right` it was fed.
///
/// # Errors
///
/// `ShapeMismatch` for an ordered probe's order that is no number, `NoSuchColumn` for one past its row.
#[expect(
    clippy::too_many_arguments,
    reason = "the left row, the right rows, the run, the probe's own scratch, its feed and its answer: one row's probe"
)]
pub(crate) fn probe_row(
    probe: &ProbeParts,
    left: InputRows<'_>,
    at: RowIndex,
    right: InputRows<'_>,
    run: &KernelRun<'_>,
    scratch: &mut ProbeScratch<'_>,
    fed: &mut NodeFed,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let walked = walked_tree(scratch.build, right)?;
    let right = if filters(probe, scratch.build) {
        InputRows::Table(&*scratch.kept)
    } else {
        right
    };
    let ProbeScratch {
        index,
        whole: [every_left, every_right],
        row,
        repeats,
        decisions,
        found: nodes,
        ..
    } = scratch;
    let (left_on, right_on): (&[ColumnIndex], &[ColumnIndex]) = match &probe.key {
        ProbeKey::Columns { left, right } => (left, right),
        ProbeKey::Whole => (every_left.as_slice(), every_right.as_slice()),
    };
    let indexed_side = KeyedSide {
        rows: right,
        on: right_on,
    };
    if probe.mode == ProbeMode::Anti
        && probe.empty_right == EmptyRight::EveryLeftRow
        && count_of(right).0 == 0
    {
        let start = CellStart(output.cells.len());
        output.cells.extend_from_slice(row_of(left, at, row));
        counted(output, start, probe.repeated, repeats);
        return Ok(None);
    }
    let matched = match walked {
        None => match find_cells(
            index,
            indexed_side,
            KeyedRowAt {
                side: KeyedSide {
                    rows: left,
                    on: left_on,
                },
                row: at,
            },
        ) {
            Ok(found) => Matched::Indexed(found),
            Err(fault) => return Ok(Some(fault)),
        },
        Some(tree) => {
            let (Some(left_key), Some(right_key)) = (left_on.first(), right_on.first()) else {
                return Err(PortableOperatorFault::WrongOperation);
            };
            let Some(key) = cell_of(left, at, *left_key) else {
                return Ok(Some(PortableOperatorFault::NoSuchColumn {
                    index: *left_key,
                }));
            };
            nodes.clear();
            keyed_roots(&tree, *right_key, key, nodes);
            if let Some(slot) = fed.0.get_mut(1) {
                *slot = RowCount(slot.0.saturating_add(nodes.len()));
            }
            // The walked probe's right filter, which cannot fail (`tree_probe`), keeps or drops each row it finds.
            if !probe.filter.comparisons.is_empty() {
                let kernel = filter(&probe.filter, kinds_of(right), run.resources.texts());
                if let Some(fault) = kept_nodes(tree, kernel, run.matcher, row, decisions, nodes) {
                    return Ok(Some(fault));
                }
            }
            Matched::Walked(tree)
        }
    };
    if probe.mode == ProbeMode::Inner {
        let walked_rows;
        let (rows, mut matched_row) = match matched {
            Matched::Indexed(key) => (right, key.map_or(NO_ROW, |key| key.first)),
            Matched::Walked(tree) => {
                walked_rows = TreeRead {
                    candidates: nodes.as_slice(),
                    ..tree
                };
                (
                    InputRows::Tree(&walked_rows),
                    next_found(None, RowCount(nodes.len())),
                )
            }
        };
        while matched_row != NO_ROW {
            let start = CellStart(output.cells.len());
            for column in &probe.output {
                let cell = match column.side {
                    Side::Left => cell_of(left, at, column.column),
                    Side::Right => cell_of(rows, indexed(matched_row), column.column),
                };
                let Some(cell) = cell else {
                    output.cells.truncate(start.0);
                    return Ok(Some(PortableOperatorFault::NoSuchColumn {
                        index: column.column,
                    }));
                };
                output.cells.push(cell);
            }
            counted(output, start, probe.repeated, repeats);
            matched_row = match matched {
                Matched::Indexed(_) => next_of(index, matched_row),
                Matched::Walked(_) => next_found(Some(matched_row), RowCount(nodes.len())),
            };
        }
        return Ok(None);
    }
    if let ProbeMode::Ordered(order) = probe.mode {
        // An ordered probe keeps its index (`tree_probe`): it never walks.
        let Matched::Indexed(found) = matched else {
            return Err(PortableOperatorFault::WrongOperation);
        };
        let passed = match found {
            None => false,
            Some(key) => {
                let bound = number_at(right, indexed(key.first), order.right_order)?;
                let own = number_at(left, at, order.left_order)?;
                match order.extremum {
                    PartitionExtremum::Maximum => bound > own,
                    PartitionExtremum::Minimum => bound < own,
                }
            }
        };
        if !passed {
            let start = CellStart(output.cells.len());
            output.cells.extend_from_slice(row_of(left, at, row));
            counted(output, start, probe.repeated, repeats);
        }
        return Ok(None);
    }
    let exists = match (probe.existence, matched) {
        (_, Matched::Walked(_)) => !nodes.is_empty(),
        (_, Matched::Indexed(None)) => false,
        (Existence::First, Matched::Indexed(Some(_))) => true,
        (Existence::Every, Matched::Indexed(Some(key))) => {
            let mut rows = RowCount(0);
            let mut matched_row = key.first;
            while matched_row != NO_ROW {
                rows = RowCount(rows.0.saturating_add(1));
                matched_row = next_of(index, matched_row);
            }
            rows.0 > 0
        }
    };
    if exists == (probe.mode == ProbeMode::Semi) {
        let start = CellStart(output.cells.len());
        output.cells.extend_from_slice(row_of(left, at, row));
        counted(output, start, probe.repeated, repeats);
    }
    Ok(None)
}

/// Appends the answer of each row of `left` probing the rows of `right` to `output`, and answers the first row's fault: the
/// right filter's, before any answer, or a left row's, after the answers of the rows before it. A left of no row reaches no
/// right row: it answers its layout before any right key or filter is read. A blocking kernel's join over rows it made (a
/// seeded closure's): a streamed probe's rows are pushed through `probe_row` by the chain of streamed stages.
///
/// # Errors
///
/// `NoSuchColumn` for an answer column, or a right key column, past its rows.
pub(crate) fn probed(
    probe: &ProbeParts,
    left: InputRows<'_>,
    right: InputRows<'_>,
    run: &KernelRun<'_>,
    mut scratch: ProbeScratch<'_>,
    fed: &mut NodeFed,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    output.kinds = answer_kinds(probe, kinds_of(left), kinds_of(right))?;
    let walked = walked_tree(scratch.build, right)?.is_some();
    if walked && let Some(slot) = fed.0.get_mut(1) {
        *slot = RowCount(0);
    }
    if count_of(left).0 == 0 {
        return Ok(None);
    }
    if let Some(fault) = probe_prepared(probe, kinds_of(left), right, run, &mut scratch)?.fault {
        return Ok(Some(fault));
    }
    emptied(scratch.repeats);
    for at in 0..count_of(left).0 {
        if let Some(fault) = probe_row(
            probe,
            left,
            RowIndex(at),
            right,
            run,
            &mut scratch,
            fed,
            output,
        )? {
            return Ok(Some(fault));
        }
    }
    Ok(None)
}

/// Makes each key's first row its extremum of the order's right column: an ordered probe reads only that row of a key.
///
/// # Errors
///
/// `ShapeMismatch` for an order cell that is not a number (the rewrite proved it one).
#[expect(
    clippy::indexing_slicing,
    reason = "the chain is made together with the rows it indexes: a row past it is a bug, which panics"
)]
fn extremes(
    right: InputRows<'_>,
    order: OrderedProbe,
    index: &mut KeyedRows,
) -> Result<(), PortableOperatorFault> {
    let KeyedRows { keys, next } = index;
    for key in keys.iter_mut() {
        let mut best = key.first;
        let mut bound = number_at(right, indexed(best), order.right_order)?;
        let mut row = next[usize::try_from(best.0).unwrap_or(usize::MAX)];
        while row != NO_ROW {
            let value = number_at(right, indexed(row), order.right_order)?;
            let past = match order.extremum {
                PartitionExtremum::Maximum => value > bound,
                PartitionExtremum::Minimum => value < bound,
            };
            if past {
                (best, bound) = (row, value);
            }
            row = next[usize::try_from(row.0).unwrap_or(usize::MAX)];
        }
        key.first = best;
    }
    Ok(())
}

/// The number in `column` of `row`.
///
/// # Errors
///
/// `ShapeMismatch` for a cell that is not a number, `NoSuchColumn` past the row.
fn number_at(
    rows: InputRows<'_>,
    row: RowIndex,
    column: ColumnIndex,
) -> Result<ryo_relations::domain::relations::types::Integer, PortableOperatorFault> {
    match cell_of(rows, row, column) {
        Some(Value::Number(number)) => Ok(number),
        Some(_) => Err(PortableOperatorFault::ShapeMismatch),
        None => Err(PortableOperatorFault::NoSuchColumn { index: column }),
    }
}

/// Every column of rows of `kinds`, in order, into `into`.
fn every(kinds: &[CellKind], into: &mut Vec<ColumnIndex>) {
    into.clear();
    into.extend((0..kinds.len()).map(|at| ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX))));
}
