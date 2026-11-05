//! One borrowed relation pass for active selection classes. No publication,
//! provider demand, fallback, or owner lifetime is decided in this domain.

use super::types::{
    ClassWork, Eligibility, KindCount, RuleActivity, WalkWork,
    prepared_walk::value::{ClassCount, NativeBatch, PreparedWalk},
};
use crate::domain::{
    comparing::service::holds_all,
    kind_sets::{
        service::{kind_tests, named_kinds, names_a_kind, passes},
        types::KindScratch,
    },
    operating::types::{KindNaming, Memo, NameCount, OperatorContext},
    positions::types::{ClassPlace, Slots},
    step_classes::types::StepClass,
    table_cells::{
        service::read_tree_cells,
        types::{CellsRead, ColumnPlaces, TreeForm},
    },
    table_model::{
        service::{count_of, kinds_of, read_row},
        types::{Rows, Verdict},
    },
    text_gate::service::needed_of_rest,
};
use ryo_language::domain::{
    elements::service::{kind_at, kind_names},
    symbols::service::{name_count, query_reborrow},
};
use ryo_relations::domain::relations::types::{CellKind, Flag, Position, Relation, Value};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::RowIndex;

fn class_context<'a>(
    base: &'a mut OperatorContext<'_>,
    bound: &'a [ParameterBinding],
    memo: &'a mut Vec<Memo>,
) -> OperatorContext<'a> {
    OperatorContext {
        semantic: base.semantic,
        prepared_kinds: base.prepared_kinds,
        prepared_matchers: base.prepared_matchers,
        bindings: bound,
        symbols: query_reborrow(&mut base.symbols),
        source: base.source,
        paths: base.paths,
        arenas: base.arenas,
        memo,
        scratch: &mut *base.scratch,
    }
}

/// The columns looked up for all eligible classes in one kind group.
fn needed_together(union: &mut ColumnPlaces, work: &[ClassWork], places: &[ClassPlace]) {
    union.0.clear();
    for place in places {
        if let Some(each) = work.get(place.0) {
            union.0.extend_from_slice(&each.needed.0);
        }
    }
    union.0.sort_unstable();
    union.0.dedup();
}

#[expect(
    clippy::too_many_arguments,
    reason = "One borrowed source row and one independent class result"
)]
fn consider(
    row: &[Value],
    class: &StepClass,
    each: &mut ClassWork,
    batch: &mut NativeBatch,
    place: ClassPlace,
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
    batch.record_visit(place);
    let mut class_context = class_context(context, bound, &mut each.memo);
    match holds_all(
        &class.comparisons,
        row,
        source_kinds,
        names,
        &mut class_context,
    ) {
        Ok(Verdict::Holds) => {
            if let Some(done) = batch.output(place) {
                done.0.push(position.0);
            }
        }
        Ok(Verdict::Fails) => {}
        Err(error) => {
            each.eligible = Eligibility::Declined;
            batch.set(place, Err(error));
        }
    }
}

/// Evaluate independent active classes in one pass. `None` declines before any
/// semantic comparison; an error after reaching a class is retained as that
/// class's result and never asks the caller to run it again.
#[expect(
    clippy::too_many_arguments,
    reason = "The prepared request and one borrowed typed source population"
)]
pub(crate) fn evaluate(
    program: &PreparedWalk,
    relation: Relation,
    rows: &Rows<'_>,
    bindings: &[Box<[ParameterBinding]>],
    context: &mut OperatorContext<'_>,
    active: &[RuleActivity],
    scratch: &mut WalkWork,
) -> NativeBatch {
    let mut batch = NativeBatch::empty(ClassCount(program.classes().len()));
    if active.len() != program.rule_count().0
        || !program.classes().iter().any(|class| {
            class.source == relation
                && class
                    .members
                    .iter()
                    .any(|member| active[member.rule.0] == RuleActivity::Active)
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
        || count > u32::MAX as usize
        || crate::domain::table_model::collections::representations::service::borrowed(rows)
            .validate()
            .is_err()
    {
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
        _ => None,
    };

    let WalkWork {
        work,
        by_kind,
        columns_by_kind,
        every,
        cells,
    } = scratch;
    work.resize_with(program.classes().len(), ClassWork::default);
    for each in work.iter_mut() {
        each.kinds = KindCount(0);
        each.rest.0.clear();
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

    for (at, class) in program.classes().iter().enumerate() {
        // Distinct predicates share the source traversal too. Duplicate consumers
        // are not required; only a class with no demanded member is excluded.
        if class.source != relation
            || !class
                .members
                .iter()
                .any(|member| active[member.rule.0] == RuleActivity::Active)
        {
            continue;
        }
        let (Some(bound), Some(each)) = (bindings.get(class.first.rule.0), work.get_mut(at)) else {
            continue;
        };
        let comparisons = class.comparisons.as_slice();
        let mut prepared = class_context(context, bound, &mut each.memo);
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
            kind_tests(tree, &comparisons[..prefix], &mut prepared, &mut kinds);
        }
        each.rest.0.extend(0..comparisons.len());
        needed_of_rest(comparisons, &each.rest, &mut each.needed);
        if each
            .needed
            .0
            .iter()
            .any(|column| *column >= source_kinds.len())
        {
            continue;
        }
        prepared.memo.clear();
        // Do not allocate a comparisons-by-symbols matrix for every class.
        // Matcher compilation and keyed verdict reuse already live in shared scratch.
        if let Some(named) = tree.and_then(|_| named_kinds(&kinds)) {
            for kind in named {
                if passes(&kinds, *kind) == Verdict::Holds
                    && let Some(group) = by_kind.get_mut(usize::from(kind.0))
                {
                    group.push(ClassPlace(at));
                    each.kinds.0 += 1;
                }
            }
        } else {
            every.push(ClassPlace(at));
        }
        each.eligible = Eligibility::Ready;
        batch.set(ClassPlace(at), Ok(Slots::default()));
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
        return batch;
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
        for place in group {
            if let (Some(class), Some(each)) =
                (program.classes().get(place.0), work.get_mut(place.0))
            {
                consider(
                    row,
                    class,
                    each,
                    &mut batch,
                    *place,
                    position,
                    names,
                    source_kinds,
                    bindings,
                    context,
                );
            }
        }
    }
    for (at, each) in work.iter().enumerate() {
        if each.kinds.0 > 1
            && let Some(done) = batch.output(ClassPlace(at))
        {
            done.0.sort_unstable();
        }
    }
    batch
}
