//! The `walks` service: the lowered step that walks the tree from the rows of another step, with no IO and no ports.

use super::constants::MARK_OF_ELEMENT;
use ryo_language::domain::elements::service::{
    children, kind_names, next_sibling, previous_sibling,
};
use ryo_language::domain::symbols::service::name_count;
use ryo_relations::domain::relations::types::{Flag, Position, Value};

use super::types::{Admission, Admit, FilterScratch, Marked};
use crate::domain::comparing::service::holds_some;
use crate::domain::kind_sets::service::{kind_tests, passes};
use crate::domain::kind_sets::types::KindScratch;
use crate::domain::lowering::service::{shape_at, step_of};
use crate::domain::lowering::types::Lowering;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Memo, NameCount, OperatorContext};
use crate::domain::positions::service::{count, followed, in_tree, made_of, row};
use crate::domain::positions::types::{LoweredRoom, Positions, Slots, Work};
use crate::domain::table_cells::service::read_tree_cells;
use crate::domain::table_cells::types::{CellsRead, TreeRows};
use crate::domain::table_model::service::{kinds_of, positions};
use crate::domain::table_model::types::{Rows, Verdict};
use crate::domain::table_scratch::types::{ElementIndex, KindsBy};
use crate::domain::text_gate::service::{needed_of_rest, text_admitted, text_gate, undecided};
use ryo_language::domain::elements::service::{kind_at, parent_at, size_at};
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Comparison, StepId};
use ryo_values::domain::plans::types::Operation;

/// Takes the scratch a filter of `comparisons` works with out of `context`, with nothing made ready yet.
pub(crate) fn filter_of<'x>(
    comparisons: &'x [Comparison],
    context: &mut OperatorContext<'_>,
) -> FilterScratch<'x> {
    FilterScratch {
        kinds: KindScratch {
            names: std::mem::take(&mut context.scratch.select_scratch.kind_names),
            sets: std::mem::take(&mut context.scratch.select_scratch.kind_sets),
            tests: std::mem::take(&mut context.scratch.select_scratch.kind_tests),
        },
        needed: std::mem::take(&mut context.scratch.needed),
        cells: [Value::Flag(Flag::Lowered); 5],
        names: NameCount(name_count(context.symbols.as_ref()).0),
        comparisons,
        gate: std::mem::take(&mut context.scratch.select_scratch.gate),
        text_ok: std::mem::take(&mut context.scratch.select_scratch.text_ok),
        rest: std::mem::take(&mut context.scratch.select_scratch.rest),
    }
}

/// Takes what deciding the elements of `tree` by `comparisons` works with from the scratch of `context`, and makes ready what the
/// comparisons decide.
pub(crate) fn take_filter<'x>(
    tree: &TreeRows<'_>,
    comparisons: &'x [Comparison],
    context: &mut OperatorContext<'_>,
) -> FilterScratch<'x> {
    let mut filter = filter_of(comparisons, context);
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
pub(crate) fn give_filter(filter: FilterScratch<'_>, context: &mut OperatorContext<'_>) {
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
#[expect(
    clippy::expect_used,
    reason = "the marks hold one for each element of the tree the axis walks; an element without one is a bug that must crash, not an element read as unmarked"
)]
pub(crate) fn admits(
    admission: &Admission<'_>,
    filter: &mut FilterScratch<'_>,
    tree: &TreeRows<'_>,
    found: Position,
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    match admission {
        Admission::Marked(marks) => Ok(
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

/// The elements of the tree the rows of the step `among` lead to by their column `column_id`: the domain of a walk that is a step.
///
/// # Errors
///
/// `NotRun` when the step was not lowered or has no such column.
fn marked_by(
    tree: &TreeRows<'_>,
    lowering: &Lowering,
    made: &[Positions],
    among: StepId,
    column_id: ColumnIndex,
) -> Result<Marked, OperatingError> {
    let column = shape_at(lowering, among)?
        .columns
        .get(usize::try_from(column_id.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(OperatingError::NotRun)?;
    let rows = made_of(made, among).ok_or(OperatingError::NotRun)?;
    let mut marks = Marked(vec![false; tree.elements.kind.len()]);
    for at in positions(count(rows)) {
        if let Some(hit) = followed(tree, column, row(rows, at))
            && let Some(mark) = marks
                .0
                .get_mut(usize::try_from(hit.0).unwrap_or(usize::MAX))
        {
            *mark = true;
        }
    }
    Ok(marks)
}

/// The rows of a walk from the rows of a lowered step.
///
/// # Errors
///
/// What a comparison refuses, or `NotRun` when the step is not a walk the tree can run.
pub(crate) fn axis(
    tree: &TreeRows<'_>,
    lowering: &Lowering,
    operation: &Operation,
    work: Work<'_, '_>,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let out = &mut out.0;
    let Work { room, context, .. } = work;
    let Operation::Axis {
        from,
        domain,
        direction,
        id: start_column,
        domain_id,
        domain_filter,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let from_id = step_of(*from)?;
    let from_shape = shape_at(lowering, from_id)?;
    let LoweredRoom { made, found, .. } = room;
    let from_rows = made_of(made, from_id).ok_or(OperatingError::NotRun)?;
    let mut admit: Admit<'_> = match domain {
        Input::Relation(_) => {
            if domain_filter.is_empty() {
                Admit::All
            } else {
                Admit::Filter(take_filter(tree, domain_filter, context), domain_filter)
            }
        }
        Input::Step(_) => Admit::Marks(marked_by(
            tree,
            lowering,
            made,
            step_of(*domain)?,
            *domain_id,
        )?),
    };
    let start_col = from_shape
        .columns
        .get(usize::try_from(start_column.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(OperatingError::NotRun)?;
    let elements = tree.elements;
    let up = |at: Position| (at.0 != 0).then(|| parent_at(elements, at));
    for at in positions(count(from_rows)) {
        let each = row(from_rows, at);
        let Some(start) = followed(tree, start_col, each) else {
            continue;
        };
        let start = in_tree(tree, start);
        found.0.clear();
        match direction {
            Direction::Parent => found.0.extend(up(start).map(|parent| parent.0)),
            Direction::Child => found
                .0
                .extend(children(elements, start).map(|child| child.0)),
            Direction::Ancestor | Direction::NearestAncestor => {
                found.0.extend(
                    std::iter::successors(up(start), |above| up(*above)).map(|ancestor| ancestor.0),
                );
            }
            Direction::Descendant => {
                found
                    .0
                    .extend(start.0 + 1..=start.0 + size_at(elements, start).0);
            }
            Direction::Deep => found.0.extend(
                (start.0 + 1..=start.0 + size_at(elements, start).0)
                    .filter(|below| parent_at(elements, Position(*below)) != start),
            ),
            Direction::PreviousSibling => found
                .0
                .extend(previous_sibling(elements, start).map(|sibling| sibling.0)),
            Direction::NextSibling => found
                .0
                .extend(next_sibling(elements, start).map(|sibling| sibling.0)),
        }
        for reached in &found.0 {
            match allows(&mut admit, tree, Position(*reached), context) {
                Ok(Verdict::Holds) => {}
                Ok(Verdict::Fails) => continue,
                Err(error) => {
                    give(admit, context);
                    return Err(error);
                }
            }
            out.extend_from_slice(each.0);
            out.push(*reached);
            if *direction == Direction::NearestAncestor {
                break;
            }
        }
    }
    give(admit, context);
    Ok(())
}

use ryo_values::domain::rows::types::RowIndex;
