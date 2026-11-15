//! One borrowed relation pass for active selection classes. No publication,
//! provider demand, fallback, or owner lifetime is decided in this domain.

use super::types::{
    ClassCount, ClassWork, CohortPlace, Eligibility, KindCount, PredicateMemo, PredicatePlace,
    WalkWork, native_batch::value::NativeBatch, prepared_walk::value::PreparedWalk,
};
use crate::domain::operator::types::OperatorContext;
use crate::domain::{
    kind_sets::{
        service::{kind_tests, named_kinds, names_a_kind, passes},
        types::KindScratch,
    },
    operating::types::KindNaming,
    shared_walk::types::Slots,
    step_classes::types::ClassPlace,
    step_classes::types::StepClass,
    table_cells::{
        service::read_tree_cells,
        types::{CellsRead, ColumnPlaces},
    },
    table_model::{
        service::{count_of, kinds_of, read_row},
        types::Rows,
    },
};
use ryo_language::domain::elements::service::{kind_at, kind_names};
use ryo_language::domain::names::service::name_count;
use ryo_operators::domain::comparing::types::NameCount;
use ryo_operators::domain::comparing::{
    service::holds_bound,
    types::{DecisionWorkspace, RowShape},
};
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::table_texts::types::Texts;
use ryo_operators::domain::tree_cells::types::TreeForm;
use ryo_relations::domain::relations::types::{CellKind, Flag, Position, Relation, Value};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::RowIndex;

/// The columns looked up for all eligible classes in one kind group.
fn needed_together(union: &mut ColumnPlaces, work: &[ClassWork], places: &[CohortPlace]) {
    union.0.clear();
    for place in places {
        if let Some(each) = work.get(place.0) {
            union.0.extend_from_slice(&each.needed.0);
        }
    }
    union.0.sort_unstable();
    union.0.dedup();
}

/// The dense slot of a predicate; a slot first reached is recorded so `forget` clears only those.
fn memo_slot(
    memo: &mut PredicateMemo,
    predicate: PredicatePlace,
) -> &mut Option<Result<PredicateOutcome, crate::domain::operating::errors::OperatingError>> {
    if memo.slots.len() <= predicate.0 {
        memo.slots.resize_with(predicate.0 + 1, || None);
    }
    if memo.slots[predicate.0].is_none() {
        memo.filled.push(predicate);
    }
    &mut memo.slots[predicate.0]
}

/// Forget every decision before the next source row.
fn forget(memo: &mut PredicateMemo) {
    for predicate in memo.filled.drain(..) {
        memo.slots[predicate.0] = None;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "One borrowed source row and one independent class result"
)]
fn consider(
    row: &[Value],
    program: &PreparedWalk,
    place: ClassPlace,
    class: &StepClass,
    predicates: &mut PredicateMemo,
    batch: &mut NativeBatch,
    each: &mut ClassWork,
    position: Position,
    names: NameCount,
    source_kinds: &[CellKind],
    bindings: &[Box<[ParameterBinding]>],
    context: &mut OperatorContext<'_>,
) {
    if each.eligible != Eligibility::Ready {
        return;
    }
    let Some(bound) = bindings.get(class.first.rule.0) else {
        return;
    };
    each.visits.0 = each.visits.0.saturating_add(1);
    let texts = Texts {
        symbols: context.symbols.as_ref(),
        source: context.source,
        paths: context.paths,
        arenas: context.arenas,
    };
    let mut verdict = Ok(PredicateOutcome::Holds);
    for &predicate in program.prefixes(place) {
        let mut tested = false;
        let decision = memo_slot(predicates, predicate).get_or_insert_with(|| {
            tested = true;
            batch.record_predicate_test();
            holds_bound(
                std::slice::from_ref(program.comparison(predicate)),
                row,
                RowShape {
                    kinds: source_kinds,
                    names,
                },
                &texts,
                bound,
                context.prepared_matchers,
                context.matcher,
                DecisionWorkspace {
                    select: &mut context.scratch.select_scratch,
                    memo: &mut each.memo,
                },
            )
            .map_err(crate::domain::operating::errors::OperatingError::from)
        });
        if !tested {
            batch.record_predicate_reuse();
        }
        if *decision != Ok(PredicateOutcome::Holds) {
            verdict = decision.clone();
            break;
        }
    }
    match verdict {
        Ok(PredicateOutcome::Holds) => {
            if let Some(result) = &mut each.result {
                result.positions.0.push(position.0);
            }
        }
        Ok(PredicateOutcome::Fails) => {}
        Err(error) => {
            each.eligible = Eligibility::Declined;
            if let Some(result) = &mut each.result {
                result.completion = Err(error);
            }
        }
    }
}

/// Evaluate independent active classes in one pass. `None` declines before any
/// semantic comparison; an error after reaching a class is retained as that
/// class's result and never asks the caller to run it again.
pub fn evaluate(
    program: &PreparedWalk,
    relation: Relation,
    rows: &Rows<'_>,
    bindings: &[Box<[ParameterBinding]>],
    context: &mut OperatorContext<'_>,
    ready: &[ClassPlace],
    scratch: &mut WalkWork,
) -> NativeBatch {
    let mut batch = NativeBatch::empty(ClassCount(program.classes().len()));
    if ready.is_empty()
        || ready.iter().any(|place| {
            program
                .classes()
                .get(place.0)
                .is_none_or(|class| class.source != relation)
        })
    {
        return batch;
    }
    let source_kinds = kinds_of(rows);
    let count = count_of(rows).0;
    // Representation/schema failures decline before any semantic predicate work.
    // Ordinary selection remains responsible for its representation-specific error.
    if source_kinds != ryo_relations::domain::relations::service::kinds(relation)
        || matches!(rows, Rows::Tree(tree) if count != 0 && kind_names(tree.elements).is_empty())
        || u32::try_from(count).is_err()
        || crate::domain::table_model::service::borrowed(rows)
            .validate()
            .is_err()
    {
        batch.refuse_representation();
        return batch;
    }
    let tree = match rows {
        Rows::Tree(tree)
            if relation == Relation::SyntaxTree
                && tree.form == TreeForm::Elements
                && !kind_names(tree.elements).is_empty() =>
        {
            Some(tree)
        }
        Rows::Tree(_) | Rows::Stored(_) | Rows::Computed(_) | Rows::Compact(_) => None,
    };

    let WalkWork {
        active,
        work,
        by_kind,
        columns_by_kind,
        every,
        cells,
        predicates,
    } = scratch;
    active.clear();
    active.extend_from_slice(ready);
    active.sort_unstable_by_key(|class| class.0);
    active.dedup();
    work.resize_with(active.len(), ClassWork::default);
    for each in work.iter_mut() {
        batch.record_scratch_reset();
        each.result = None;
        each.visits = Default::default();
        each.kinds = KindCount(0);
        each.needed.0.clear();
        each.memo.clear();
        each.eligible = Eligibility::Declined;
    }
    cells.resize(source_kinds.len(), Value::Flag(Flag::Lowered));
    by_kind.resize_with(
        tree.map_or(0, |tree| kind_names(tree.elements).len()),
        Vec::new,
    );
    for group in by_kind.iter_mut() {
        group.clear();
    }
    every.clear();
    let names = NameCount(name_count(context.symbols.as_ref()).0);
    let mut kinds = KindScratch {
        names: std::mem::take(&mut context.scratch.select_scratch.kind_names),
        sets: std::mem::take(&mut context.scratch.select_scratch.kind_sets),
        tests: std::mem::take(&mut context.scratch.select_scratch.kind_tests),
    };

    for (at, place) in active.iter().enumerate() {
        // Dense scratch is cohort-local; compiled class identities remain immutable.
        let class = &program.classes()[place.0];
        let (Some(_), Some(each)) = (bindings.get(class.first.rule.0), work.get_mut(at)) else {
            continue;
        };
        let comparisons = class.comparisons.as_slice();
        // Only an initial sequence of total kind equalities may narrow dispatch.
        // Every predicate is still evaluated in authored order for admitted rows.
        let prefix = comparisons
            .iter()
            .take_while(|comparison| {
                names_a_kind(comparison) == KindNaming::Names
                    && matches!(
                        comparison.test,
                        ryo_values::domain::plan_parts::types::Test::Equal
                            | ryo_values::domain::plan_parts::types::Test::NotEqual
                    )
            })
            .count();
        if let Some(tree) = tree {
            kind_tests(
                tree,
                &comparisons[..prefix],
                context.prepared_kinds,
                context.matcher,
                context.symbols.as_ref(),
                context.source,
                &mut kinds,
            );
        }
        each.needed.0.extend_from_slice(&program.needed(*place).0);
        if each
            .needed
            .0
            .iter()
            .any(|column| *column >= source_kinds.len())
        {
            continue;
        }
        each.memo.clear();
        // Do not allocate a comparisons-by-symbols matrix for every class.
        // Matcher compilation and keyed verdict reuse already live in shared scratch.
        if let Some(named) = tree.and_then(|_| named_kinds(&kinds)) {
            for kind in named {
                if passes(&kinds, *kind) == PredicateOutcome::Holds
                    && let Some(group) = by_kind.get_mut(usize::from(kind.0))
                {
                    group.push(CohortPlace(at));
                    each.kinds.0 += 1;
                }
            }
        } else {
            every.push(CohortPlace(at));
        }
        each.eligible = Eligibility::Ready;
        each.result = Some(super::types::ClassResult {
            positions: Slots::default(),
            completion: Ok(()),
        });
    }

    context.scratch.select_scratch.kind_names = kinds.names;
    context.scratch.select_scratch.kind_sets = kinds.sets;
    context.scratch.select_scratch.kind_tests = kinds.tests;

    // The vocabulary dispatch is prepared once, but rows themselves are visited
    // in source order. This preserves the first semantic error of each class.
    columns_by_kind.resize_with(by_kind.len(), ColumnPlaces::default);
    for (group, columns) in by_kind.iter_mut().zip(columns_by_kind.iter_mut()) {
        group.extend(every.iter().copied());
        group.sort_unstable_by_key(|place| place.0);
        group.dedup();
        needed_together(columns, work, group);
    }
    if by_kind.iter().all(Vec::is_empty) && every.is_empty() {
        return finish(batch, active, work);
    }
    batch.record_pass();
    // Every representation uses this same source-ordered outer walk. Only the
    // exact native SyntaxTree form has vocabulary dispatch and selective decoding.
    for index in 0..count {
        batch.record_source_row();
        let position = Position(u32::try_from(index).unwrap_or(u32::MAX));
        let (group, columns) = if let Some(tree) = tree {
            let kind = usize::from(kind_at(tree.elements, position).0);
            let (Some(group), Some(columns)) = (by_kind.get(kind), columns_by_kind.get(kind))
            else {
                continue;
            };
            (group.as_slice(), Some(columns))
        } else {
            (every.as_slice(), None)
        };
        if group.is_empty() {
            continue;
        }
        let row = if let (Some(tree), Some(columns)) = (tree, columns) {
            if read_tree_cells(tree, RowIndex(index), cells, columns) == CellsRead::Missing {
                continue;
            }
            cells.as_slice()
        } else {
            let Some(row) = read_row(rows, RowIndex(index), cells) else {
                continue;
            };
            row
        };
        batch.record_decoded_row();
        forget(predicates);
        for place in group {
            if let (Some(class), Some(each)) = (
                program.classes().get(active[place.0].0),
                work.get_mut(place.0),
            ) {
                consider(
                    row,
                    program,
                    active[place.0],
                    class,
                    predicates,
                    &mut batch,
                    each,
                    position,
                    names,
                    source_kinds,
                    bindings,
                    context,
                );
            }
        }
    }
    finish(batch, active, work)
}

fn finish(mut batch: NativeBatch, active: &[ClassPlace], work: &mut [ClassWork]) -> NativeBatch {
    for (place, each) in active.iter().copied().zip(work) {
        if let Some(mut result) = each.result.take() {
            if each.kinds.0 > 1 {
                result.positions.0.sort_unstable();
            }
            batch.store(place, result, each.visits);
        }
    }
    batch
}
