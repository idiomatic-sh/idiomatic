//! One borrowed tree pass for equivalent selection classes. No publication,
//! provider demand, fallback, or owner lifetime is decided in this domain.

use super::types::{
    ClassWork, Eligibility, KindCount, RowRead, RuleActivity, WalkWork,
    prepared_walk::value::{ClassCount, NativeBatch, PreparedWalk},
};
use crate::domain::{
    comparing::service::holds_some,
    coverage::types::RuleIndex,
    kind_sets::{
        service::{kind_tests, named_kinds, passes},
        types::KindScratch,
    },
    operating::{
        errors::OperatingError,
        types::{Memo, NameCount, OperatorContext},
    },
    positions::types::{ClassPlace, Slots},
    step_classes::{constants::ELEMENT_COLUMNS, types::StepClass},
    table_cells::{
        service::read_tree_cells,
        types::{CellsRead, ColumnPlaces, TreeForm, TreeRows},
    },
    table_model::{
        service::kinds_of,
        types::{Rows, Verdict},
    },
    table_scratch::types::{ElementIndex, KindsBy},
    text_gate::service::{needed_of_rest, text_admitted, text_gate, undecided},
};
use ryo_language::domain::{
    elements::service::{kind_at, kind_names, positions, real_count},
    symbols::service::{name_count, query_reborrow},
};
use ryo_relations::domain::relations::types::{CellKind, Position, Value};
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
    reason = "One borrowed tree row and one independent class result"
)]
fn consider(
    tree: &TreeRows<'_>,
    class: &StepClass,
    each: &mut ClassWork,
    batch: &mut NativeBatch,
    place: ClassPlace,
    position: Position,
    union: &ColumnPlaces,
    names: NameCount,
    element_kinds: &[CellKind],
    bindings: &[Box<[ParameterBinding]>],
    context: &mut OperatorContext<'_>,
    cells: &mut [Value; ELEMENT_COLUMNS],
    read: &mut RowRead,
) {
    if each.eligible != Eligibility::Ready {
        return;
    }
    let Some(bound) = bindings.get(class.first.rule.0) else {
        return;
    };
    batch.record_visit(place);
    let comparisons = class.comparisons.as_slice();
    let index = usize::try_from(position.0).unwrap_or(usize::MAX);
    let mut class_context = class_context(context, bound, &mut each.memo);
    let decided = (|| -> Result<Verdict, OperatingError> {
        if !each.gate.0.is_empty()
            && text_admitted(
                tree,
                ElementIndex(index),
                comparisons,
                &each.gate,
                &mut each.text_ok,
                names,
                cells,
                element_kinds,
                &mut class_context,
            )? == Verdict::Fails
        {
            return Ok(Verdict::Fails);
        }
        if *read == RowRead::Pending {
            if read_tree_cells(tree, RowIndex(index), cells, union) == CellsRead::Missing {
                return Ok(Verdict::Fails);
            }
            *read = RowRead::Done;
        }
        holds_some(
            comparisons,
            &each.rest,
            cells,
            element_kinds,
            names,
            &mut class_context,
        )
    })();
    match decided {
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
pub(crate) fn evaluate(
    program: &PreparedWalk,
    tree: TreeRows<'_>,
    bindings: &[Box<[ParameterBinding]>],
    context: &mut OperatorContext<'_>,
    active: &[RuleActivity],
    scratch: &mut WalkWork,
) -> NativeBatch {
    let mut batch = NativeBatch::empty(ClassCount(program.classes().len()));
    if tree.form != TreeForm::Elements
        || real_count(tree.elements).0 > u32::MAX as usize
        || kind_names(tree.elements).is_empty()
        || program.classes().is_empty()
        || active.len() != program.rule_count().0
    {
        return batch;
    }

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
        each.gate.0.clear();
        each.text_ok.0.clear();
        each.rest.0.clear();
        each.needed.0.clear();
        each.memo.clear();
        each.eligible = Eligibility::Declined;
    }
    by_kind.resize_with(kind_names(tree.elements).len(), Vec::new);
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
        // A single active member has no shared work. Its ordinary invocation
        // remains the only owner of its result and error.
        if class
            .members
            .iter()
            .filter(|member| active[member.rule.0] == RuleActivity::Active)
            .nth(1)
            .is_none()
        {
            continue;
        }
        let (Some(bound), Some(each)) = (bindings.get(class.first.rule.0), work.get_mut(at)) else {
            continue;
        };
        let comparisons = class.comparisons.as_slice();
        let mut prepared = class_context(context, bound, &mut each.memo);
        kind_tests(&tree, comparisons, &mut prepared, &mut kinds);
        text_gate(&tree, comparisons, &mut each.gate);
        each.text_ok.0.clear();
        if !each.gate.0.is_empty() {
            each.text_ok.0.resize(names.0 + 1, 0);
        }
        undecided(comparisons, &each.gate, KindsBy::Sets, &mut each.rest);
        needed_of_rest(comparisons, &each.rest, &mut each.needed);
        if each
            .needed
            .0
            .iter()
            .any(|column| *column >= ELEMENT_COLUMNS)
        {
            continue;
        }
        prepared.memo.clear();
        if !each.rest.0.is_empty() {
            prepared
                .memo
                .resize(comparisons.len() * names.0, Memo::Unknown);
        }
        if let Some(named) = named_kinds(&kinds) {
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
    let rows = Rows::Tree(tree);
    let element_kinds = kinds_of(&rows);

    // The vocabulary dispatch is prepared once, but rows themselves are visited
    // in source order. This preserves the first semantic error of each class.
    columns_by_kind.resize_with(by_kind.len(), ColumnPlaces::default);
    for (group, columns) in by_kind.iter_mut().zip(columns_by_kind.iter_mut()) {
        group.extend(every.iter().copied());
        group.sort_unstable_by_key(|place| place.0);
        group.dedup();
        needed_together(columns, work, group);
    }
    for position in positions(tree.elements) {
        let kind = usize::from(kind_at(tree.elements, position).0);
        let (Some(group), Some(columns)) = (by_kind.get(kind), columns_by_kind.get(kind)) else {
            continue;
        };
        let mut read = RowRead::Pending;
        for place in group {
            if let (Some(class), Some(each)) =
                (program.classes().get(place.0), work.get_mut(place.0))
            {
                consider(
                    &tree,
                    class,
                    each,
                    &mut batch,
                    *place,
                    position,
                    columns,
                    names,
                    element_kinds,
                    bindings,
                    context,
                    cells,
                    &mut read,
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
