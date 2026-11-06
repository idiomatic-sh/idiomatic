//! The `walks` service: the lowered step that walks the tree from the rows of another step, with no IO and no ports.

use super::constants::MARK_OF_ELEMENT;
use ryo_language::domain::elements::service::kind_names;
use ryo_language::domain::symbols::service::name_count;
use ryo_relations::domain::relations::types::{Flag, Position, Value};

use super::types::{Admission, Admit, FilterScratch};
use crate::domain::comparing::service::holds_some;
use crate::domain::kind_sets::service::{kind_tests, passes};
use crate::domain::kind_sets::types::KindScratch;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Memo, NameCount, OperatorContext};
use crate::domain::table_cells::service::read_tree_cells;
use crate::domain::table_cells::types::{CellsRead, TreeRows};
use crate::domain::table_model::service::kinds_of;
use crate::domain::table_model::types::{Rows, Verdict};
use crate::domain::table_scratch::types::{ElementIndex, KindsBy};
use crate::domain::text_gate::service::{needed_of_rest, text_admitted, text_gate, undecided};
use ryo_language::domain::elements::service::kind_at;
use ryo_values::domain::plan_parts::types::PlanComparison;

/// Takes filter scratch out of `context`, with nothing made ready yet.
pub(crate) fn filter_of(context: &mut OperatorContext<'_>) -> FilterScratch {
    FilterScratch {
        kinds: KindScratch {
            names: std::mem::take(&mut context.scratch.select_scratch.kind_names),
            sets: std::mem::take(&mut context.scratch.select_scratch.kind_sets),
            tests: std::mem::take(&mut context.scratch.select_scratch.kind_tests),
        },
        needed: std::mem::take(&mut context.scratch.needed),
        cells: [Value::Flag(Flag::Lowered); 5],
        names: NameCount(name_count(context.symbols.as_ref()).0),
        gate: std::mem::take(&mut context.scratch.select_scratch.gate),
        text_ok: std::mem::take(&mut context.scratch.select_scratch.text_ok),
        rest: std::mem::take(&mut context.scratch.select_scratch.rest),
    }
}

/// Takes what deciding the elements of `tree` by `comparisons` works with from the scratch of `context`, and makes ready what the
/// comparisons decide.
pub(crate) fn take_filter(
    tree: &TreeRows<'_>,
    comparisons: &[PlanComparison],
    context: &mut OperatorContext<'_>,
) -> FilterScratch {
    let mut filter = filter_of(context);
    context.memo.clear();
    context
        .memo
        .resize(comparisons.len() * filter.names.0, Memo::Unknown);
    kind_tests(tree, comparisons, context, &mut filter.kinds);
    text_gate(tree, comparisons, &mut filter.gate);
    filter.text_ok.0.clear();
    if !filter.gate.0.is_empty() {
        filter.text_ok.0.resize(filter.names.0 + 1, 0);
    }
    let kinds_by = if kind_names(tree.elements).is_empty() {
        KindsBy::Nothing
    } else {
        KindsBy::Sets
    };
    undecided(comparisons, &filter.gate, kinds_by, &mut filter.rest);
    needed_of_rest(comparisons, &filter.rest, &mut filter.needed);
    filter
}

/// Gives back what a filter took.
pub(crate) fn give_filter(filter: FilterScratch, context: &mut OperatorContext<'_>) {
    context.scratch.select_scratch.kind_names = filter.kinds.names;
    context.scratch.select_scratch.kind_sets = filter.kinds.sets;
    context.scratch.select_scratch.kind_tests = filter.kinds.tests;
    context.scratch.needed = filter.needed;
    context.scratch.select_scratch.gate = filter.gate;
    context.scratch.select_scratch.text_ok = filter.text_ok;
    context.scratch.select_scratch.rest = filter.rest;
}

/// Whether the element at `found` is an answer.
///
/// # Errors
///
/// What a comparison refuses.

pub(crate) fn admits(
    admission: &Admission<'_>,
    filter: &mut FilterScratch,
    tree: &TreeRows<'_>,
    found: Position,
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    match admission {
        Admission::Filtered(comparisons) => {
            if !filter.kinds.tests.0.is_empty()
                && passes(&filter.kinds, kind_at(tree.elements, found)) == Verdict::Fails
            {
                return Ok(Verdict::Fails);
            }
            let place = usize::try_from(found.0).unwrap_or(usize::MAX);
            if !filter.gate.0.is_empty()
                && text_admitted(
                    tree,
                    ElementIndex(place),
                    comparisons,
                    &filter.gate,
                    &mut filter.text_ok,
                    filter.names,
                    &mut filter.cells,
                    kinds_of(&Rows::Tree(*tree)),
                    context,
                )? == Verdict::Fails
            {
                return Ok(Verdict::Fails);
            }
            if filter.rest.0.is_empty() {
                return Ok(Verdict::Holds);
            }
            if read_tree_cells(tree, RowIndex(place), &mut filter.cells, &filter.needed)
                == CellsRead::Missing
            {
                return Ok(Verdict::Fails);
            }
            holds_some(
                comparisons,
                &filter.rest,
                &filter.cells,
                kinds_of(&Rows::Tree(*tree)),
                filter.names,
                context,
            )
        }
    }
}

/// Whether the element at `found` is an answer.
///
/// # Errors
///
/// What a comparison refuses.
#[expect(
    clippy::expect_used,
    reason = "the marks hold one for each element of the tree the axis walks; an element without one is a bug that must crash, not an element read as unmarked"
)]
pub(crate) fn allows(
    admit: &mut Admit<'_>,
    tree: &TreeRows<'_>,
    found: Position,
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    match admit {
        Admit::All => Ok(Verdict::Holds),
        Admit::Marks(marks) => Ok(
            if marks
                .0
                .get(usize::try_from(found.0).unwrap_or(usize::MAX))
                .copied()
                .expect(MARK_OF_ELEMENT)
            {
                Verdict::Holds
            } else {
                Verdict::Fails
            },
        ),
        Admit::Filter(filter, comparisons) => admits(
            &Admission::Filtered(comparisons),
            filter,
            tree,
            found,
            context,
        ),
    }
}

/// Gives back what was taken.
pub(crate) fn give(admit: Admit<'_>, context: &mut OperatorContext<'_>) {
    if let Admit::Filter(filter, _) = admit {
        give_filter(filter, context);
    }
}

use ryo_values::domain::rows::types::RowIndex;
