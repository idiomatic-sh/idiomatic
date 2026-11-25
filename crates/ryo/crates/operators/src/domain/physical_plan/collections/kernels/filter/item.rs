//! The `filter` item of the `kernels` collection: the rows a select's compiled comparisons, the moved ones unchanged, hold for,
//! with its kept columns.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_roots::item::tree_row;
use super::super::scan_facts::item::{decode, facts_cell_at};
use crate::domain::comparing::service::{FastComparison, fast_holds, fast_of, holds_bound_from};
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::{Bits, ColumnsView};
use crate::domain::comparing::types::{DecisionWorkspace, NameCount, RowShape};
use crate::domain::contract::types::{PortableOperatorFault, PredicateOutcome};
use crate::domain::patterns::types::PatternMatcher;
use crate::domain::physical_plan::types::TreeRead;
use crate::domain::physical_plan::types::file_buffers::value::NodeDecisions;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::select::types::SelectParts;
use crate::domain::table_scratch::service::forgotten;
use crate::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::types::{CellKind, NodeId, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `filter`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct FilterItem;

#[cfg(test)]
impl KernelsItem for FilterItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = {
            match lowering {
                Lowering::Filter { .. } | Lowering::Lookup { .. } => true,
                Lowering::Probe { probe, .. } => !probe.filter.comparisons.is_empty(),
                Lowering::SeededReach { join, .. } => !join.probe.filter.comparisons.is_empty(),
                Lowering::Axis { axis, .. } => !axis.domain.comparisons.is_empty(),
                Lowering::Project { .. }
                | Lowering::Extend { .. }
                | Lowering::Distinct { .. }
                | Lowering::Group { .. }
                | Lowering::TextMap { .. }
                | Lowering::PointerChase { .. }
                | Lowering::OrderedMatch { .. }
                | Lowering::Render { .. }
                | Lowering::Reach { .. }
                | Lowering::OrderedWalk { .. }
                | Lowering::Worklist { .. }
                | Lowering::Components { .. }
                | Lowering::ScanBound { .. }
                | Lowering::Unlowered => false,
            }
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// One filter's comparisons over rows of one shape, against the open file's texts.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FilterKernel<'f> {
    /// The comparisons, the kept columns and the bindings.
    pub(crate) select: &'f SelectParts,
    /// What each column of a row holds.
    pub(crate) kinds: &'f [CellKind],
    /// The open file's texts.
    pub(crate) texts: Texts<'f>,
    /// How many names the open file has.
    pub(crate) names: NameCount,
    /// Spike (ryo-fmx3): the first comparisons, each specialised once; the rest only the generic path decides.
    pub(crate) fast: [FastComparison<'f>; FAST],
}

/// Spike (ryo-fmx3): how many of a filter's comparisons are specialised.
pub(crate) const FAST: usize = 4;

/// The filter of `select` over rows of `kinds`, against the open file's names as they are now. Its memo is readied once per
/// node run (`readied`), never here, and grows to the places it decides.
pub(crate) fn filter<'f>(
    select: &'f SelectParts,
    kinds: &'f [CellKind],
    texts: Texts<'f>,
) -> FilterKernel<'f> {
    let mut fast = [FastComparison::Generic; FAST];
    for (slot, comparison) in fast.iter_mut().zip(&select.comparisons) {
        *slot = fast_of(comparison);
    }
    FilterKernel {
        select,
        kinds,
        texts,
        names: NameCount(ryo_language::domain::names::service::name_count(texts.symbols).0),
        fast,
    }
}

/// Readies `decisions`' memo for one run of a filter: every decision of its last run forgotten, and only those.
pub(crate) fn readied(decisions: &mut NodeDecisions) {
    forgotten(&mut decisions.memo);
}

/// Whether every comparison holds for `row`.
///
/// # Errors
///
/// `ShapeMismatch` for a row of another width; the first comparison's fault the row raises.
pub(crate) fn holds(
    filter: FilterKernel<'_>,
    matcher: PatternMatcher<'_>,
    row: &[Value],
    decisions: &mut NodeDecisions,
) -> Result<PredicateOutcome, PortableOperatorFault> {
    let comparisons = &filter.select.comparisons;
    if !comparisons.is_empty() && row.len() != filter.kinds.len() {
        return Err(PortableOperatorFault::ShapeMismatch);
    }
    holds_from(filter, matcher, 0, row, decisions)
}

/// Spike (ryo-fmx3): whether every comparison from `from` on holds for `row`, a row of the filter's width.
fn holds_from(
    filter: FilterKernel<'_>,
    matcher: PatternMatcher<'_>,
    from: usize,
    row: &[Value],
    decisions: &mut NodeDecisions,
) -> Result<PredicateOutcome, PortableOperatorFault> {
    let comparisons = &filter.select.comparisons;
    let fasts = filter.fast.get(..comparisons.len().min(FAST)).unwrap_or_default();
    holds_bound_from(
        comparisons,
        fasts,
        from,
        row,
        RowShape {
            kinds: filter.kinds,
            names: filter.names,
        },
        &filter.texts,
        &filter.select.bindings,
        matcher,
        DecisionWorkspace {
            select: &mut decisions.select,
            memo: &mut decisions.memo,
        },
    )
    .map_err(PortableOperatorFault::from)
}

/// Spike (ryo-fmx3): `keep` over row `at` of a relation's columns, read in place: the specialised comparisons read only the
/// cells they name, a row is decoded only when a comparison needs the generic path, and a kept row's cells are copied
/// straight from the columns.
///
/// # Errors
///
/// As `keep`.
#[expect(clippy::too_many_arguments, reason = "spike: keep's arguments plus the columns and the row")]
pub(crate) fn keep_facts(
    filter: FilterKernel<'_>,
    matcher: PatternMatcher<'_>,
    view: &ColumnsView<'_>,
    at: RowIndex,
    scratch: &mut Vec<Value>,
    decisions: &mut NodeDecisions,
    written: &mut Vec<Value>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let comparisons = &filter.select.comparisons;
    let mut from = comparisons.len();
    for (number, fast) in filter.fast.iter().take(comparisons.len()).enumerate() {
        match fast_holds(fast, |column| facts_cell_at(view, at, column), &filter.texts) {
            Some(PredicateOutcome::Holds) => {}
            Some(PredicateOutcome::Fails) => return Ok(()),
            None => {
                from = number;
                break;
            }
        }
    }
    if from == comparisons.len() && comparisons.len() <= FAST {
        // Every comparison held: the row is written from the columns, the kept ones or all of them.
        if filter.select.kept.is_empty() {
            output.cells.extend(
                view.kinds
                    .iter()
                    .zip(view.columns)
                    .filter_map(|(kind, column)| column.0.get(at.0).map(|bits| from_bits(*kind, Bits(*bits)))),
            );
        } else {
            let start = output.cells.len();
            for column in &filter.select.kept {
                let Some(cell) = facts_cell_at(view, at, usize::try_from(column.0).unwrap_or(usize::MAX)) else {
                    output.cells.truncate(start);
                    return Err(PortableOperatorFault::NoSuchColumn { index: *column });
                };
                output.cells.push(cell);
            }
        }
        output.count = RowCount(output.count.0 + 1);
        return Ok(());
    }
    let from = from.min(FAST);
    decode(view, at, scratch);
    let row: &[Value] = scratch;
    if !comparisons.is_empty() && row.len() != filter.kinds.len() {
        return Err(PortableOperatorFault::ShapeMismatch);
    }
    if holds_from(filter, matcher, from, row, decisions)? == PredicateOutcome::Fails {
        return Ok(());
    }
    if filter.select.kept.is_empty() {
        output.cells.extend_from_slice(row);
    } else {
        written.clear();
        crate::domain::project::service::project(row, &filter.select.kept, written)?;
        output.cells.extend_from_slice(written);
    }
    output.count = RowCount(output.count.0 + 1);
    Ok(())
}


/// Keeps the nodes of `nodes` whose rows of `tree` every comparison holds for, in order, and answers the first row's fault.
pub(crate) fn kept_nodes(
    tree: TreeRead<'_>,
    filter: FilterKernel<'_>,
    matcher: PatternMatcher<'_>,
    row: &mut Vec<Value>,
    decisions: &mut NodeDecisions,
    nodes: &mut Vec<NodeId>,
) -> Option<PortableOperatorFault> {
    let mut fault = None;
    nodes.retain(|node| {
        if fault.is_some() {
            return false;
        }
        let one = TreeRead {
            candidates: std::slice::from_ref(node),
            ..tree
        };
        tree_row(&one, RowIndex(0), row);
        match holds(filter, matcher, row, decisions) {
            Ok(outcome) => outcome != PredicateOutcome::Fails,
            Err(error) => {
                fault = Some(error);
                false
            }
        }
    });
    fault
}

/// Appends `row`, with the kept columns, to `output` when every comparison holds for it.
///
/// # Errors
///
/// `ShapeMismatch` for a row of another width; the first comparison's fault the row raises; `NoSuchColumn` for a kept column
/// past the row.
pub(crate) fn keep(
    filter: FilterKernel<'_>,
    matcher: PatternMatcher<'_>,
    row: &[Value],
    decisions: &mut NodeDecisions,
    written: &mut Vec<Value>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    if holds(filter, matcher, row, decisions)? == PredicateOutcome::Fails {
        return Ok(());
    }
    if filter.select.kept.is_empty() {
        output.cells.extend_from_slice(row);
    } else {
        written.clear();
        crate::domain::project::service::project(row, &filter.select.kept, written)?;
        output.cells.extend_from_slice(written);
    }
    output.count = RowCount(output.count.0 + 1);
    Ok(())
}

/// The kinds of the kept columns of a row of `kinds`; none kept keeps every column.
///
/// # Errors
///
/// `NoSuchColumn` for a kept column past the row.
pub(crate) fn kept_kinds(
    kinds: &[CellKind],
    kept: &[ColumnIndex],
) -> Result<Vec<CellKind>, PortableOperatorFault> {
    if kept.is_empty() {
        return Ok(kinds.to_vec());
    }
    kept.iter()
        .map(|column| {
            kinds
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })
        })
        .collect()
}
