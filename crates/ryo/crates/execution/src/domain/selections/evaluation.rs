//! The `select` operator: keep the rows for which every comparison holds.

use ryo_language::domain::elements::service::kind_names;
use ryo_language::domain::symbols::service::name_count;
use ryo_relations::domain::relations::types::{CellKind, Flag, Value};

use crate::domain::comparing::service::{holds_all, holds_some};
use crate::domain::kind_sets::service::{candidates_of, kind_tests, passes};
use crate::domain::kind_sets::types::KindScratch;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::KindNarrowing;
use crate::domain::operating::types::{Emit, InputPlace, Memo, NameCount, OperatorContext};
use crate::domain::replay::types::Collect;
use crate::domain::table_cells::service::{read_cells, read_tree_cells};
use crate::domain::table_cells::types::{CellsRead, ColumnPlaces, TreeForm, TreeRows};
use crate::domain::table_model::types::{Rows, SelectedRows, Verdict};
use crate::domain::table_scratch::types::{ElementIndex, KindsBy};
use crate::domain::text_gate::service::{
    needed_of, needed_of_rest, text_admitted, text_candidates, text_gate, undecided,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, PlanComparison};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::errors::RowsError;
use {
    crate::domain::table_model::service::cell_at, crate::domain::table_model::service::count_of,
    crate::domain::table_model::service::kinds_of, crate::domain::table_model::service::read_row,
    crate::domain::table_model::service::rectangular, ryo_values::domain::rows::service::cell,
    ryo_values::domain::rows::service::column_in, ryo_values::domain::rows::service::positions,
    ryo_values::domain::rows::service::reset,
};

/// `select`, one item of the `operators` collection.

/// What `tree_each` hands each element the comparisons admit: its place, the cells the comparisons read, and the context.
type EachElement<'e> = dyn FnMut(RowIndex, &mut Vec<Value>, &mut OperatorContext<'_>) -> Result<(), OperatingError>
    + 'e;

/// Whether every column `needed` names is one of the columns `kinds` has: the first that is not is the error.
fn columns_present(needed: &ColumnPlaces, kinds: &[CellKind]) -> Result<(), OperatingError> {
    match needed.0.iter().find(|place| **place >= kinds.len()) {
        Some(missing) => Err(OperatingError::Table(RowsError::NoSuchColumn {
            index: u32::try_from(*missing).unwrap_or(u32::MAX),
        })),
        None => Ok(()),
    }
}

/// What the kind sets decide for the elements of `tree`, which is then not asked of an element again.
fn kinds_by(tree: &TreeRows<'_>) -> KindsBy {
    if tree.form == TreeForm::Elements && !kind_names(tree.elements).is_empty() {
        KindsBy::Sets
    } else {
        KindsBy::Nothing
    }
}

/// Whether the kind sets let the element at `index` through: every element passes when no comparison names a kind.
fn kind_verdict(kinds: &KindScratch, tree: &TreeRows<'_>, index: RowIndex) -> Verdict {
    if kinds.tests.0.is_empty() {
        return Verdict::Holds;
    }
    tree.elements
        .kind
        .get(index.0)
        .map_or(Verdict::Holds, |kind| passes(kinds, *kind))
}

/// As `tree_each`, with one event for each candidate visited in source order, before its
/// comparisons can refuse. The owner uses these events for actual logical work accounting.
pub(crate) fn tree_each_with_visits(
    comparisons: &[PlanComparison],
    tree: &TreeRows<'_>,
    buffer: &mut Vec<Value>,
    context: &mut OperatorContext<'_>,
    visit: &mut dyn FnMut(RowIndex),
    each: &mut EachElement<'_>,
) -> Result<(), OperatingError> {
    let rows = Rows::Tree(*tree);
    let names = NameCount(name_count(context.symbols.as_ref()).0);
    let mut needed = std::mem::take(&mut context.scratch.needed);
    needed_of(comparisons, &mut needed);
    let present = columns_present(&needed, kinds_of(&rows));
    if present.is_err() {
        context.scratch.needed = needed;
        return present;
    }
    buffer.clear();
    buffer.resize(kinds_of(&rows).len(), Value::Flag(Flag::Lowered));
    context.memo.clear();
    context
        .memo
        .resize(comparisons.len() * names.0, Memo::Unknown);
    let mut candidates = std::mem::take(&mut context.scratch.select_scratch.candidates);
    let mut gate = std::mem::take(&mut context.scratch.select_scratch.gate);
    let mut text_ok = std::mem::take(&mut context.scratch.select_scratch.text_ok);
    text_gate(tree, comparisons, &mut gate);
    text_ok.0.clear();
    if !gate.0.is_empty() {
        text_ok.0.resize(names.0 + 1, 0);
    }
    let mut kinds = KindScratch {
        names: std::mem::take(&mut context.scratch.select_scratch.kind_names),
        sets: std::mem::take(&mut context.scratch.select_scratch.kind_sets),
        tests: std::mem::take(&mut context.scratch.select_scratch.kind_tests),
    };
    kind_tests(tree, comparisons, context, &mut kinds);
    // What the kind sets and the text gate decided for an element is not asked of it again.
    let mut rest = std::mem::take(&mut context.scratch.select_scratch.rest);
    undecided(comparisons, &gate, kinds_by(tree), &mut rest);
    needed_of_rest(comparisons, &rest, &mut needed);
    let mut narrowed = candidates_of(tree, &kinds, &mut candidates) == KindNarrowing::Narrowed;
    let outcome = (|| {
        if !narrowed && !gate.0.is_empty() {
            narrowed = text_candidates(
                tree,
                comparisons,
                &gate,
                &mut text_ok,
                names,
                buffer,
                kinds_of(&rows),
                context,
                &mut candidates,
            )? == KindNarrowing::Narrowed;
        }
        let places = if narrowed {
            candidates.0.len()
        } else {
            count_of(&rows).0
        };
        for place in 0..places {
            let index = if narrowed {
                let candidate = candidates.0.get(place).copied().unwrap_or(u32::MAX);
                RowIndex(usize::try_from(candidate).unwrap_or(usize::MAX))
            } else {
                RowIndex(place)
            };
            visit(index);
            if kind_verdict(&kinds, tree, index) == Verdict::Fails {
                continue;
            }
            if !gate.0.is_empty()
                && text_admitted(
                    tree,
                    ElementIndex(index.0),
                    comparisons,
                    &gate,
                    &mut text_ok,
                    names,
                    buffer,
                    kinds_of(&rows),
                    context,
                )? == Verdict::Fails
            {
                continue;
            }
            if read_tree_cells(tree, index, buffer, &needed) == CellsRead::Missing {
                continue;
            }
            if holds_some(comparisons, &rest, buffer, kinds_of(&rows), names, context)?
                == Verdict::Holds
            {
                each(index, buffer, context)?;
            }
        }
        Ok(())
    })();
    context.scratch.select_scratch.rest = rest;
    context.scratch.select_scratch.gate = gate;
    context.scratch.select_scratch.text_ok = text_ok;
    context.scratch.needed = needed;
    context.scratch.select_scratch.candidates = candidates;
    context.scratch.select_scratch.kind_names = kinds.names;
    context.scratch.select_scratch.kind_sets = kinds.sets;
    context.scratch.select_scratch.kind_tests = kinds.tests;
    outcome
}

use ryo_values::domain::rows::types::{RowIndex, Table};
