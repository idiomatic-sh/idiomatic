//! The `probe_right` item of the `kernels` collection: a probe's right side, readied once before its first left row and read
//! by every left row after: the right rows its filter keeps, its key columns, its index (its own, a shared one, or the tree
//! it walks) and an ordered probe's extremes, each probe's its own; and that side as a run of left rows reads it.

use super::super::filter::item::{FilterKernel, filter, keep, readied};
use super::super::hash_index::item::indexed;
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{cell_of, count_of, kinds_of, row_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::{
    EmptyRight, OrderedProbe, PartitionExtremum, ProbeKey, ProbeMode, ProbeParts,
};
use crate::domain::patterns::types::PatternMatcher;
use crate::domain::physical_plan::types::file_buffers::value::NodeDecisions;
use crate::domain::physical_plan::types::keyed_rows::value::{KeyedRows, NO_ROW};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::row_set::value::RowSet;
use crate::domain::physical_plan::types::row_set::value::columns;
use crate::domain::physical_plan::types::shared_index::value::IndexRows;
use crate::domain::physical_plan::types::{InputRows, KernelRun, KeyedSide, NodeFed, TreeRead};
use ryo_relations::domain::relations::types::{CellKind, NodeId, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `probe_right`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ProbeRightItem;

#[cfg(test)]
impl KernelsItem for ProbeRightItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = matches!(
            lowering,
            Lowering::Probe { .. } | Lowering::SeededReach { .. }
        );
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
    /// The index it reads, of the kind its key needs: its own, or a shared index.
    pub(crate) index: &'s mut IndexRows,
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
    pub(crate) repeats: &'s mut RowSet,
    /// The right filter's decisions.
    pub(crate) decisions: &'s mut NodeDecisions,
    /// The tree nodes one left row found, for a probe that walks the tree.
    pub(crate) found: &'s mut Vec<NodeId>,
}

/// Whether a probe probes each left row, or keeps every one unread: an anti probe of no right row that keeps them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LeftRows {
    /// Each left row is probed.
    Probed,
    /// Every left row is kept, none probed.
    Kept,
}

/// A probe's right side as its left rows read it, after `probe_prepared` readied it: the right rows it reads (the rows its
/// filter kept, when it filters), the tree it walks, its key columns, its index, a walked probe's filter of the rows it
/// finds, and the scratch and feed every left row writes through.
pub(crate) struct ProbeReads<'a> {
    /// The right rows it reads.
    pub(crate) right: InputRows<'a>,
    /// The tree it walks; none for a probe of an index.
    pub(crate) walked: Option<TreeRead<'a>>,
    /// Its key columns: its left's, then its right's.
    pub(crate) keys: [&'a [ColumnIndex]; 2],
    /// Its index of the right rows, of the kind its key needs.
    pub(crate) index: &'a IndexRows,
    /// The row read, decoded.
    pub(crate) row: &'a mut Vec<Value>,
    /// The answers kept, keyed by their cells, when repeats are dropped.
    pub(crate) repeats: &'a mut RowSet,
    /// The right filter's decisions.
    pub(crate) decisions: &'a mut NodeDecisions,
    /// The tree nodes one left row found.
    pub(crate) found: &'a mut Vec<NodeId>,
    /// A walked probe's filter of the rows it finds; none when it filters none.
    pub(crate) kept: Option<FilterKernel<'a>>,
    /// The matching semantics.
    pub(crate) matcher: PatternMatcher<'a>,
    /// What it was fed: a walked probe's found rows are its right rows.
    pub(crate) fed: &'a mut NodeFed,
    /// Whether it probes each left row.
    pub(crate) left: LeftRows,
    /// Spike (ryo-fmx3): whether the `fastkey` lever is on.
    pub(crate) fast_key: bool,
}

/// The right side of `probe` over `right`, readied in `scratch`, as its left rows read it: resolved once for a run of left
/// rows, never per row.
///
/// # Errors
///
/// `WrongOperation` for a walked probe whose right rows are not the tree's.
pub(crate) fn probe_reads<'a>(
    probe: &'a ProbeParts,
    right: InputRows<'a>,
    run: &'a KernelRun<'_>,
    scratch: &'a mut ProbeScratch<'_>,
    fed: &'a mut NodeFed,
) -> Result<ProbeReads<'a>, PortableOperatorFault> {
    let walked = walked_tree(scratch.build, right)?;
    let build = scratch.build;
    let ProbeScratch {
        index,
        kept,
        whole: [every_left, every_right],
        row,
        repeats,
        decisions,
        found,
        ..
    } = scratch;
    let right = read_right(probe, build, right, kept);
    Ok(ProbeReads {
        right,
        walked,
        keys: match &probe.key {
            ProbeKey::Columns { left, right } => [left, right],
            ProbeKey::Whole => [every_left.as_slice(), every_right.as_slice()],
        },
        index,
        row,
        repeats,
        decisions,
        found,
        // The walked probe's right filter, which cannot fail (`tree_probe`), keeps or drops each row it finds.
        kept: (walked.is_some() && !probe.filter.comparisons.is_empty())
            .then(|| filter(&probe.filter, kinds_of(right), run.resources.texts())),
        matcher: run.matcher,
        fed,
        left: if probe.mode == ProbeMode::Anti
            && probe.empty_right == EmptyRight::EveryLeftRow
            && count_of(right).0 == 0
        {
            LeftRows::Kept
        } else {
            LeftRows::Probed
        },
        fast_key: crate::spike::probe_lever("fastkey"),
    })
}

/// The tree a walked probe reads its right rows from in place; none for a probe of an index.
///
/// # Errors
///
/// `WrongOperation` for a walked probe whose right rows are not the tree's.
pub(crate) fn walked_tree<'a>(
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

/// The right rows a probe reads: the rows its filter kept, when it filters and does not walk the tree, else `right`.
pub(crate) fn read_right<'a>(
    probe: &ProbeParts,
    build: IndexBuild,
    right: InputRows<'a>,
    kept: &'a Table,
) -> InputRows<'a> {
    if probe.filter.comparisons.is_empty() || build == IndexBuild::Walked {
        right
    } else {
        InputRows::Table(kept)
    }
}

/// The probe's work before it reads its first left row, in its held order: its right filter over `right` into its kept rows,
/// its key columns, its index build and an ordered probe's extremes; answers the right filter's fault, which stops the probe
/// before any answer. A walked probe readies its found-row filter. A probe whose left has no row does none of it.
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
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let walked = walked_tree(scratch.build, right)?;
    scratch.kept.kinds.clear();
    scratch.kept.cells.clear();
    scratch.kept.count = RowCount(0);
    if !probe.filter.comparisons.is_empty() && walked.is_none() {
        let kinds = kinds_of(right);
        let kernel = filter(&probe.filter, kinds, run.resources.texts());
        readied(scratch.decisions);
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
                return Ok(Some(fault));
            }
        }
    }
    if walked.is_some() && !probe.filter.comparisons.is_empty() {
        readied(scratch.decisions);
    }
    let right = read_right(probe, scratch.build, right, scratch.kept);
    let [left_on, right_on] = &mut scratch.whole;
    if matches!(probe.key, ProbeKey::Whole) {
        every(left, left_on);
        every(kinds_of(right), right_on);
    }
    let on = match &probe.key {
        ProbeKey::Columns { right, .. } => right.as_slice(),
        ProbeKey::Whole => right_on.as_slice(),
    };
    if scratch.build == IndexBuild::Builds {
        // A whole-row key reads a row set, any other key keyed rows: the index was readied of its key's kind.
        let built = match (&probe.key, &mut *scratch.index) {
            (ProbeKey::Columns { .. }, IndexRows::Keyed(rows)) => {
                // Spike (ryo-fmx3): each right row read by its own kind's reader.
                super::super::hash_index::item::cells_typed(KeyedSide { rows: right, on }, rows)
            }
            (ProbeKey::Whole, IndexRows::Rows(set)) => {
                set.built(count_of(right), kinds_of(right), |row, column| {
                    cell_of(right, row, column)
                })
            }
            (ProbeKey::Columns { .. }, IndexRows::Rows(_))
            | (ProbeKey::Whole, IndexRows::Keyed(_)) => Err(PortableOperatorFault::WrongOperation),
        };
        if let Err(fault) = built {
            // A partial index is no index: the next probe of a shared one builds it again.
            scratch.index.cleared();
            return Err(fault);
        }
    }
    // An ordered probe keeps its own index (`shared_indexes`), which it builds: its keys' first rows become their extremes.
    if let ProbeMode::Ordered(order) = probe.mode {
        let IndexRows::Keyed(rows) = &mut *scratch.index else {
            return Err(PortableOperatorFault::WrongOperation);
        };
        extremes(right, order, rows)?;
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
pub(crate) fn number_at(
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
    into.extend(columns(kinds));
}
