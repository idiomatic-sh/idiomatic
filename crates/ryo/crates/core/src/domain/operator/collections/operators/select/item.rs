//! The `select` operator: keep the rows for which every comparison holds.

use ryo_language::domain::elements::service::kind_names;
use ryo_language::domain::relations::types::{CellKind, Flag, Value};
use ryo_language::domain::symbols::service::name_count;

use super::super::interface::OperatorsItem;
use crate::domain::comparing::service::{holds_all, holds_some};
use crate::domain::kind_sets::service::{candidates_of, kind_tests, passes};
use crate::domain::kind_sets::types::KindScratch;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::types::KindNarrowing;
use crate::domain::operating::types::{
    Emit, Feed, InputPlace, Memo, NameCount, OperatorContext, Room, Supply,
};
use crate::domain::replay::service::derive;
use crate::domain::replay::types::{Collect, Pipe};
use crate::domain::table_cells::service::{read_cells, read_tree_cells};
use crate::domain::table_cells::types::{CellsRead, ColumnPlaces, RowIndex, TreeForm, TreeRows};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{
    cell, column_in, count_of, kinds_of, positions, read_row, rectangular, reset,
};
use crate::domain::table_model::types::{Rows, Table, Verdict};
use crate::domain::table_scratch::types::{ElementIndex, KindsBy};
use crate::domain::text_gate::service::{
    needed_of, needed_of_rest, text_admitted, text_candidates, text_gate, undecided,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Comparison};
use ryo_values::domain::plans::types::Operation;

/// `select`, one item of the `operators` collection.
pub(crate) struct SelectItem;

/// Passes a row on, cut to the columns kept, or whole when none are named.
fn emit_kept(
    emit: &mut dyn Emit,
    context: &mut OperatorContext<'_>,
    row: &[Value],
    kept: &[ColumnIndex],
    cut: &mut Vec<Value>,
) -> Result<(), OperatingError> {
    if kept.is_empty() {
        return emit.row(context, row);
    }
    cut.clear();
    for index in kept {
        cut.push(cell(row, *index)?);
    }
    emit.row(context, cut)
}

impl OperatorsItem for SelectItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Select { kept, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let rows = first(inputs)?;
        let _names = NameCount(name_count(context.symbols).0);
        if count_of(rows).0 == 0 {
            reset(out, kinds_of(rows));
            if !kept.is_empty() {
                out.kinds.clear();
                for index in kept {
                    out.kinds.push(
                        *column_in(kinds_of(rows), *index)
                            .ok_or(TableModelError::NoSuchColumn { index: index.0 })?,
                    );
                }
            }
            return Ok(());
        }
        reset(out, kinds_of(rows));
        if !kept.is_empty() {
            out.kinds.clear();
            for index in kept {
                out.kinds.push(
                    *column_in(kinds_of(rows), *index)
                        .ok_or(TableModelError::NoSuchColumn { index: index.0 })?,
                );
            }
        }
        select_rows(operation, rows, context, &mut Collect(out))
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let Operation::Select {
            comparisons, kept, ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Room {
            memo,
            scratch,
            held,
            kinds,
            ..
        } = room;
        // Rows to read in place: a relation, or the table of a step that keeps its rows.
        if ins.streams(InputPlace(0)) == Supply::AtHand {
            let rows = ins.hold(InputPlace(0), context, &mut held[0])?;
            let mut own = derive(context, memo, scratch);
            return select_rows(operation, &rows, &mut own, emit);
        }
        // Rows as they are made: each is decided as it comes.
        kinds.clear();
        kinds.extend_from_slice(ins.kinds(InputPlace(0))?);
        let names = NameCount(name_count(context.symbols).0);
        let mut cut = std::mem::take(&mut scratch.other);
        let mut ready = false;
        let outcome = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                // What is decided about the names is made ready when the first row comes.
                if !ready {
                    memo.clear();
                    memo.resize(comparisons.len() * names.0, Memo::Unknown);
                    ready = true;
                }
                let mut own = derive(each, memo, scratch);
                if holds_all(comparisons, row, kinds, names, &mut own)? == Verdict::Holds {
                    emit_kept(emit, &mut own, row, kept, &mut cut)?;
                }
                Ok(())
            }),
        );
        scratch.other = cut;
        outcome
    }
}

/// What `tree_each` hands each element the comparisons admit: its place, the cells the comparisons read, and the context.
type EachElement<'e> = dyn FnMut(RowIndex, &mut Vec<Value>, &mut OperatorContext<'_>) -> Result<(), OperatingError>
    + 'e;

/// Whether every column `needed` names is one of the columns `kinds` has: the first that is not is the error.
fn columns_present(needed: &ColumnPlaces, kinds: &[CellKind]) -> Result<(), OperatingError> {
    match needed.0.iter().find(|place| **place >= kinds.len()) {
        Some(missing) => Err(OperatingError::Table(TableModelError::NoSuchColumn {
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

/// Passes each element of a tree the comparisons admit to `each`, in the order of the tree for the elements found without a pass
/// over it, with the cells the comparisons read left in `buffer`. Elements are found among the kinds the comparisons name, or among
/// the texts they admit, when few enough; the whole tree is looked at otherwise.
///
/// # Errors
///
/// A comparison that asks for a column the tree does not have, or what `each` refuses.
pub(crate) fn tree_each(
    comparisons: &[Comparison],
    tree: &TreeRows<'_>,
    buffer: &mut Vec<Value>,
    context: &mut OperatorContext<'_>,
    each: &mut EachElement<'_>,
) -> Result<(), OperatingError> {
    let rows = Rows::Tree(*tree);
    let names = NameCount(name_count(context.symbols).0);
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

/// Passes on the rows of `rows` for which every comparison of the select holds, cut to the columns it keeps. `rows` are read in
/// place: a relation's, a tree's, or a table's.
///
/// # Errors
///
/// Whatever the comparisons or the one that takes the rows refuse.
pub(crate) fn select_rows(
    operation: &Operation,
    rows: &Rows<'_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let Operation::Select {
        comparisons, kept, ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let names = NameCount(name_count(context.symbols).0);
    if count_of(rows).0 == 0 {
        return Ok(());
    }
    context.memo.clear();
    context
        .memo
        .resize(comparisons.len() * names.0, Memo::Unknown);
    let mut cut = std::mem::take(&mut context.scratch.other);
    let mut buffer = std::mem::take(&mut context.scratch.row);
    let outcome = match rows {
        // A stored row is decided on the cells the comparisons read, and read whole only if it holds.
        Rows::Stored(view) => {
            let projected = !kept.is_empty()
                && rectangular(*view)
                && kept
                    .iter()
                    .all(|column| (column.0 as usize) < view.kinds.len());
            let projection = ColumnPlaces(kept.iter().map(|column| column.0 as usize).collect());
            let mut needed = std::mem::take(&mut context.scratch.needed);
            needed_of(comparisons, &mut needed);
            if let Some(missing) = needed
                .0
                .iter()
                .find(|place| **place >= view.kinds.len())
                .copied()
            {
                context.scratch.needed = needed;
                context.scratch.row = buffer;
                return Err(OperatingError::Table(TableModelError::NoSuchColumn {
                    index: u32::try_from(missing).unwrap_or(u32::MAX),
                }));
            }
            buffer.clear();
            buffer.resize(view.kinds.len(), Value::Flag(Flag::Lowered));
            let outcome = positions(count_of(rows)).try_for_each(|index| {
                if read_cells(view, index, &mut buffer, &needed) == CellsRead::Missing {
                    return Ok(());
                }
                if holds_all(comparisons, &buffer, view.kinds, names, context)? == Verdict::Holds {
                    if projected {
                        if read_cells(view, index, &mut buffer, &projection) == CellsRead::Read {
                            emit_kept(emit, context, &buffer, kept, &mut cut)?;
                        }
                        return Ok(());
                    }
                    let Some(row) = read_row(rows, index, &mut buffer) else {
                        return Ok(());
                    };
                    emit_kept(emit, context, row, kept, &mut cut)?;
                    // `read_row` leaves the row whole; the next row starts from a buffer as wide as the rows.
                }
                Ok(())
            });
            context.scratch.needed = needed;
            outcome
        }
        Rows::Tree(tree) => tree_each(
            comparisons,
            tree,
            &mut buffer,
            context,
            &mut |index: RowIndex,
                  row_buffer: &mut Vec<Value>,
                  walked: &mut OperatorContext<'_>| {
                let Some(row) = read_row(rows, index, row_buffer) else {
                    return Ok(());
                };
                emit_kept(emit, walked, row, kept, &mut cut)
            },
        ),
        Rows::Computed(_) | Rows::Selected(_) => positions(count_of(rows)).try_for_each(|index| {
            let Some(row) = read_row(rows, index, &mut buffer) else {
                return Ok(());
            };
            if holds_all(comparisons, row, kinds_of(rows), names, context)? == Verdict::Holds {
                emit_kept(emit, context, row, kept, &mut cut)?;
            }
            Ok(())
        }),
    };
    context.scratch.row = buffer;
    context.scratch.other = cut;
    outcome
}
