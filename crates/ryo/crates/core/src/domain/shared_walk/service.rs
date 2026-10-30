//! The `shared_walk` service: the `select` steps of the call's rules over the elements of a file, decided in one pass over them, with
//! no IO and no ports. The classes of the steps it decides are the `step_classes` concept's.

use super::types::{
    ClassWork, ElementReading, KindCount, Merged, MergedWalk, Merging, Pass, Walking,
};
use crate::domain::comparing::service::holds_some;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::finish::types::RuleNumber;
use crate::domain::kind_sets::service::{kind_tests, named_kinds, passes};
use crate::domain::kind_sets::types::KindScratch;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Memo, NameCount, OperatorContext};
use crate::domain::operator::types::TreeSteps;
use crate::domain::placement::service::governs;
use crate::domain::placement::types::Governing;
use crate::domain::positions::types::{ClassPlace, LoweredRoom};
use crate::domain::step_classes::constants::ELEMENT_COLUMNS;
use crate::domain::step_classes::service::merge;
use crate::domain::step_classes::types::StepClass;
use crate::domain::table_cells::service::read_tree_cells;
use crate::domain::table_cells::types::ColumnPlaces;
use crate::domain::table_cells::types::{CellsRead, RowIndex, TreeForm, TreeRows};
use crate::domain::table_model::service::kinds_of;
use crate::domain::table_model::types::{Rows, Verdict};
use crate::domain::table_scratch::types::{ElementIndex, KindsBy};
use crate::domain::table_texts::types::SourceView;
use crate::domain::text_gate::service::{needed_of_rest, text_admitted, text_gate, undecided};
use ryo_language::domain::elements::service::{kind_names, positions};
use ryo_language::domain::kind_index::service::of_kind;
use ryo_language::domain::relations::types::{Flag, KindBits, Position, Value};
use ryo_language::domain::symbols::service::name_count;

/// Readies the room for the steps of the rule `rule`: which of them the walk has decided.
pub(crate) fn install(shared: &MergedWalk, room: &mut LoweredRoom, rule: RuleIndex) {
    room.picked.clear();
    if let Some(of_rule) = shared.merge.class_of.get(rule.0) {
        room.picked.extend(of_rule.iter().map(|class| {
            class.filter(|place| {
                shared
                    .work
                    .get(place.0)
                    .is_some_and(|work| work.merged == Merged::Together)
            })
        }));
    }
}

/// Decides whether the element at `position` is selected by the class at `place`, and keeps it when it is: its text first, by the
/// verdict the class already has of it, then the remaining comparisons over the cells the element's classes read together. A
/// comparison that refuses leaves the class to its steps.
fn consider(
    pass: &mut Pass<'_>,
    union: &ColumnPlaces,
    class: &StepClass,
    each: &mut ClassWork,
    place: ClassPlace,
    position: Position,
) {
    let Some(bound) = pass.bindings.get(class.first.rule.0) else {
        return;
    };
    let comparisons = class.comparisons.as_slice();
    let index = usize::try_from(position.0).unwrap_or(usize::MAX);
    let mut context = OperatorContext {
        bindings: bound,
        symbols: &mut *pass.symbols,
        source: pass.source,
        paths: pass.paths,
        arenas: &[],
        memo: &mut each.memo,
        scratch: &mut *pass.scratch,
    };
    let decided = (|| -> Result<Verdict, OperatingError> {
        if !each.gate.0.is_empty()
            && text_admitted(
                &pass.tree,
                ElementIndex(index),
                comparisons,
                &each.gate,
                &mut each.text_ok,
                pass.names,
                &mut pass.cells,
                pass.element_kinds,
                &mut context,
            )? == Verdict::Fails
        {
            return Ok(Verdict::Fails);
        }
        if pass.read == ElementReading::NotYet {
            if read_tree_cells(&pass.tree, RowIndex(index), &mut pass.cells, union)
                == CellsRead::Missing
            {
                return Ok(Verdict::Fails);
            }
            pass.read = ElementReading::Done;
        }
        holds_some(
            comparisons,
            &each.rest,
            &pass.cells,
            pass.element_kinds,
            pass.names,
            &mut context,
        )
    })();
    match decided {
        Ok(Verdict::Holds) => {
            if let Some(done) = pass.room.classes.get_mut(place.0) {
                done.0.push(position.0);
            }
        }
        Ok(Verdict::Fails) => {}
        Err(_) => each.merged = Merged::Apart,
    }
}

/// The columns the classes at `places` read, together, each once.
fn needed_together(union: &mut ColumnPlaces, work: &[ClassWork], places: &[ClassPlace]) {
    union.0.clear();
    for place in places {
        union.0.extend(
            work.get(place.0)
                .map(|each| each.needed.0.as_slice())
                .unwrap_or_default(),
        );
    }
    union.0.sort_unstable();
    union.0.dedup();
}

/// Decides, in one pass over the elements of the file, the steps that select from them for the rules that govern it.
///
/// A class that names kinds is decided over the elements of those kinds; one that names none, over every element; each element is
/// read once for the classes that look at it. A class whose comparisons the walk cannot decide is left to its steps; so is every
/// class when the steps run as operators. A comparison that refuses leaves its class to its steps, which then refuse in their place.
#[expect(
    clippy::too_many_lines,
    reason = "the pass is one sequence over the same borrows of the worker's tables: readying each class, dispatching the kinds, visiting the elements; a part would borrow them again"
)]
pub(crate) fn walk(shared: &mut MergedWalk, walking: Walking<'_>) {
    let Walking {
        plans,
        bindings,
        paths,
        mask,
        file,
        elements,
        symbols,
        source,
        scratch,
        room,
        steps,
    } = walking;
    if shared.made == Merging::NotYet {
        shared.merge = merge(plans, bindings);
        shared.work.clear();
        shared
            .work
            .resize_with(shared.merge.classes.len(), ClassWork::default);
        shared.made = Merging::Done;
    }
    for work in &mut shared.work {
        work.merged = Merged::Apart;
        work.kinds = KindCount(0);
    }
    room.picked.clear();
    let kinds_in_file = kind_names(elements).len();
    if steps != TreeSteps::Lowered || kinds_in_file == 0 || shared.merge.classes.is_empty() {
        return;
    }
    room.classes
        .resize_with(shared.merge.classes.len(), Default::default);
    for each in &mut room.classes {
        each.0.clear();
    }
    let MergedWalk {
        merge,
        work,
        by_kind,
        every,
        union,
        ..
    } = shared;
    for each in by_kind.iter_mut() {
        each.clear();
    }
    by_kind.resize_with(kinds_in_file, Vec::new);
    every.clear();
    let tree = TreeRows {
        elements,
        file,
        form: TreeForm::Elements,
    };
    let names = NameCount(name_count(symbols).0);
    let mut kinds = KindScratch {
        names: std::mem::take(&mut scratch.kind_names),
        sets: std::mem::take(&mut scratch.kind_sets),
        tests: std::mem::take(&mut scratch.kind_tests),
    };
    // Ready each class a rule that governs the file has a step of.
    for (place, class) in merge.classes.iter().enumerate() {
        // A class that one rule's step alone is in has nothing to share: the step decides it on its own.
        if class
            .members
            .iter()
            .filter(|member| {
                governs(
                    mask,
                    RuleNumber(u32::try_from(member.rule.0).unwrap_or(u32::MAX)),
                ) == Governing::Governs
            })
            .nth(1)
            .is_none()
        {
            continue;
        }
        let comparisons = class.comparisons.as_slice();
        let (Some(bound), Some(each)) = (bindings.get(class.first.rule.0), work.get_mut(place))
        else {
            continue;
        };
        let mut context = OperatorContext {
            bindings: bound,
            symbols: &mut *symbols,
            source: SourceView(source.0),
            paths,
            arenas: &[],
            memo: &mut each.memo,
            scratch: &mut *scratch,
        };
        kind_tests(&tree, comparisons, &mut context, &mut kinds);
        text_gate(&tree, comparisons, &mut each.gate);
        each.text_ok.0.clear();
        if !each.gate.0.is_empty() {
            each.text_ok.0.resize(names.0 + 1, 0);
        }
        undecided(comparisons, &each.gate, KindsBy::Sets, &mut each.rest);
        needed_of_rest(comparisons, &each.rest, &mut each.needed);
        // A column the elements do not have is refused by the steps themselves.
        if each
            .needed
            .0
            .iter()
            .any(|column| *column >= ELEMENT_COLUMNS)
        {
            continue;
        }
        // What was decided of the names of another file means nothing here: it is emptied, and made ready only for the comparisons
        // that are asked of the elements one by one.
        context.memo.clear();
        if !each.rest.0.is_empty() {
            context
                .memo
                .resize(comparisons.len() * names.0, Memo::Unknown);
        }
        if let Some(named) = named_kinds(&kinds) {
            for kind in named {
                if passes(&kinds, *kind) == Verdict::Holds
                    && let Some(group) = by_kind.get_mut(usize::from(kind.0))
                {
                    group.push(ClassPlace(place));
                    each.kinds.0 += 1;
                }
            }
        } else {
            every.push(ClassPlace(place));
        }
        each.merged = Merged::Together;
    }
    scratch.kind_names = kinds.names;
    scratch.kind_sets = kinds.sets;
    scratch.kind_tests = kinds.tests;
    let element_kinds = kinds_of(&Rows::Tree(tree));
    let mut pass = Pass {
        bindings,
        paths,
        tree,
        names,
        element_kinds,
        source,
        symbols,
        scratch,
        room,
        cells: [Value::Flag(Flag::Lowered); ELEMENT_COLUMNS],
        read: ElementReading::NotYet,
    };
    for (kind, group) in by_kind.iter().enumerate() {
        if group.is_empty() {
            continue;
        }
        needed_together(union, work, group);
        let kind = KindBits(u16::try_from(kind).unwrap_or(u16::MAX));
        for position in of_kind(elements, kind) {
            pass.read = ElementReading::NotYet;
            for place in group {
                if let (Some(class), Some(each)) =
                    (merge.classes.get(place.0), work.get_mut(place.0))
                    && each.merged == Merged::Together
                {
                    consider(&mut pass, union, class, each, *place, *position);
                }
            }
        }
    }
    if !every.is_empty() {
        needed_together(union, work, every);
        for position in positions(elements) {
            pass.read = ElementReading::NotYet;
            for place in every.iter() {
                if let (Some(class), Some(each)) =
                    (merge.classes.get(place.0), work.get_mut(place.0))
                    && each.merged == Merged::Together
                {
                    consider(&mut pass, union, class, each, *place, position);
                }
            }
        }
    }
    for (place, each) in work.iter().enumerate() {
        if each.merged == Merged::Together
            && each.kinds.0 > 1
            && let Some(done) = pass.room.classes.get_mut(place)
        {
            done.0.sort_unstable();
        }
    }
}
