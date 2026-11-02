//! The `operator` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It walks a plan: each step reads its inputs, and the operator its operation names
//! computes the rows. Every dispatch is a `match` that names every operation.

use super::collections::operators::antijoin::item::AntijoinItem;
use super::collections::operators::axis::item::AxisItem;
use super::collections::operators::capture::item::CaptureItem;
use super::collections::operators::closure::item::ClosureItem;
use super::collections::operators::cycles::item::CyclesItem;
use super::collections::operators::difference::item::DifferenceItem;
use super::collections::operators::extend::item::ExtendItem;
use super::collections::operators::group_count::item::GroupCountItem;
use super::collections::operators::group_digest::item::GroupDigestItem;
use super::collections::operators::interface::OperatorsItem as _;
use super::collections::operators::intersection::item::IntersectionItem;
use super::collections::operators::join::item::JoinItem;
use super::collections::operators::project::item::ProjectItem;
use super::collections::operators::rank::item::RankItem;
use super::collections::operators::rows::item::RowsItem;
use super::collections::operators::select::item::SelectItem;
use super::collections::operators::span_between::item::SpanBetweenItem;
use super::collections::operators::union::item::UnionItem;
use super::collections::operators::words::item::WordsItem;
use super::collections::operators::{
    ordered_dfs::item::OrderedDfsItem, ordered_match::item::OrderedMatchItem,
    replace::item::ReplaceItem,
};
use super::types::{Pool, RowsFed};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, OperatorContext, Room};

use crate::domain::coverage::service::drop_uncertain;
use crate::domain::coverage::types::Partials;
use crate::domain::table_model::service::{copy_table, count_of, kinds_of};
use crate::domain::table_model::types::{Frontier, RowCount, Rows, Table};
use crate::domain::trust::service::{files_for, needs_of};
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::service::inputs_of;
use ryo_values::domain::plans::service::named;
use ryo_values::domain::plans::service::step_scope;
use ryo_values::domain::plans::types::{Operation, Plan, StepScope};

/// Computes one step with the operator its operation is, into `out`, whose room it reuses.
///
/// # Errors
///
/// The operator a step's operation is, asked to pass on its rows.
pub(crate) fn operate(
    operation: &Operation,
    ins: &mut dyn Feed<'_>,
    context: &mut OperatorContext<'_>,
    room: &mut Room,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    match operation {
        Operation::Select { .. } => SelectItem.stream(operation, ins, context, room, emit),
        Operation::Project { .. } => ProjectItem.stream(operation, ins, context, room, emit),
        Operation::Extend { .. } => ExtendItem.stream(operation, ins, context, room, emit),
        Operation::Join { .. } => JoinItem.stream(operation, ins, context, room, emit),
        Operation::Union { .. } => UnionItem.stream(operation, ins, context, room, emit),
        Operation::Intersection { .. } => {
            IntersectionItem.stream(operation, ins, context, room, emit)
        }
        Operation::Difference { .. } => DifferenceItem.stream(operation, ins, context, room, emit),
        Operation::Antijoin { .. } => AntijoinItem.stream(operation, ins, context, room, emit),
        Operation::Replace { .. } => ReplaceItem.stream(operation, ins, context, room, emit),
        Operation::SpanBetween { .. } => {
            SpanBetweenItem.stream(operation, ins, context, room, emit)
        }
        Operation::OrderedMatch { .. } => {
            OrderedMatchItem.stream(operation, ins, context, room, emit)
        }
        Operation::OrderedDfs { .. } => OrderedDfsItem.stream(operation, ins, context, room, emit),
        Operation::GroupCount { .. } => GroupCountItem.stream(operation, ins, context, room, emit),
        Operation::GroupDigest { .. } => {
            GroupDigestItem.stream(operation, ins, context, room, emit)
        }
        Operation::Capture { .. } => CaptureItem.stream(operation, ins, context, room, emit),
        Operation::Words { .. } => WordsItem.stream(operation, ins, context, room, emit),
        Operation::Rank { .. } => RankItem.stream(operation, ins, context, room, emit),
        Operation::Axis { .. } => AxisItem.stream(operation, ins, context, room, emit),
        Operation::Closure { .. } => ClosureItem.stream(operation, ins, context, room, emit),
        Operation::Cycles { .. } => CyclesItem.stream(operation, ins, context, room, emit),
        Operation::Rows { .. } => RowsItem.stream(operation, ins, context, room, emit),
        // The host's rows are made once, after the files: no file's steps run them.
        Operation::Hosted { .. } => Err(OperatingError::WrongOperation),
    }
}

/// Whatever the operator refuses.
pub(crate) fn compute(
    operation: &Operation,
    inputs: &[Rows<'_>],
    context: &mut OperatorContext<'_>,
    out: &mut Table,
) -> Result<(), OperatingError> {
    match operation {
        Operation::Select { .. } => SelectItem.compute(operation, inputs, context, out),
        Operation::Project { .. } => ProjectItem.compute(operation, inputs, context, out),
        Operation::Extend { .. } => ExtendItem.compute(operation, inputs, context, out),
        Operation::Join { .. } => JoinItem.compute(operation, inputs, context, out),
        Operation::Union { .. } => UnionItem.compute(operation, inputs, context, out),
        Operation::Intersection { .. } => IntersectionItem.compute(operation, inputs, context, out),
        Operation::Difference { .. } => DifferenceItem.compute(operation, inputs, context, out),
        Operation::Antijoin { .. } => AntijoinItem.compute(operation, inputs, context, out),
        Operation::Replace { .. } => ReplaceItem.compute(operation, inputs, context, out),
        Operation::SpanBetween { .. } => SpanBetweenItem.compute(operation, inputs, context, out),
        Operation::OrderedMatch { .. } => OrderedMatchItem.compute(operation, inputs, context, out),
        Operation::OrderedDfs { .. } => OrderedDfsItem.compute(operation, inputs, context, out),
        Operation::GroupCount { .. } => GroupCountItem.compute(operation, inputs, context, out),
        Operation::GroupDigest { .. } => GroupDigestItem.compute(operation, inputs, context, out),
        Operation::Capture { .. } => CaptureItem.compute(operation, inputs, context, out),
        Operation::Words { .. } => WordsItem.compute(operation, inputs, context, out),
        Operation::Rank { .. } => RankItem.compute(operation, inputs, context, out),
        Operation::Axis { .. } => AxisItem.compute(operation, inputs, context, out),
        Operation::Closure { .. } => ClosureItem.compute(operation, inputs, context, out),
        Operation::Cycles { .. } => CyclesItem.compute(operation, inputs, context, out),
        Operation::Rows { .. } => RowsItem.compute(operation, inputs, context, out),
        // The host's rows are put in place by the finish, which has them; no operator makes them.
        Operation::Hosted { .. } => Err(OperatingError::WrongOperation),
    }
}

/// A pool big enough for any of the plans, which run one after another on it: each plan's steps and
/// inputs are emptied and filled again, so what one leaves is not read by the next.
#[must_use]
pub(crate) fn pool_for(plans: &[Plan]) -> Pool {
    let steps = plans.iter().map(|plan| plan.steps.len()).max().unwrap_or(0);
    let frontier = plans
        .iter()
        .map(|plan| plan.frontier.len())
        .max()
        .unwrap_or(0);
    Pool {
        steps: vec![Table::default(); steps],
        frontier: vec![Table::default(); frontier],
        frontier_pending: vec![false; frontier],
        ..Pool::default()
    }
}

/// Gives back what a huge file made the pool's tables grow past the room a worker keeps.
pub(crate) fn trim_pool(pool: &mut Pool) {
    crate::domain::working_set::service::clear_rule(pool);
}

/// What a step that reads nothing reads.
static NOTHING: Table = Table {
    kinds: Vec::new(),
    cells: Vec::new(),
    count: RowCount(0),
};

/// The rows the plan emits for the file just evaluated when the emitting step runs per file.
#[must_use]
pub(crate) fn emitted<'p>(plan: &Plan, pool: &'p mut Pool) -> Option<&'p mut Table> {
    match step_scope(plan, plan.emit.step) {
        Some(StepScope::PerFile) => pool
            .steps
            .get_mut(usize::try_from(plan.emit.step.0).unwrap_or(usize::MAX)),
        Some(StepScope::CrossFile) | None => None,
    }
}

/// Own the handed frontier through cross-file evaluation without making a second resident copy.
/// Runs the cross-file steps of a plan in order; frontier ownership ends when evaluation returns.
///
/// # Errors
///
/// Whatever an operator refuses, or `NotRun`/`NotRead` when a step reads what nothing provides.
pub(crate) fn evaluate_cross(
    plan: &Plan,
    frontier: Vec<Frontier>,
    partials: &Partials,
    context: &mut OperatorContext<'_>,
    fed: &mut [RowsFed],
    hosted: &[Table],
) -> Result<Table, OperatingError> {
    let needs = needs_of(plan);
    let mut computed: Vec<Table> = vec![Table::default(); plan.steps.len()];
    // The step that reads each step last: when it is done, the rows of the step it read are not kept.
    let mut last: Vec<Option<usize>> = vec![None; plan.steps.len()];
    for (index, step) in plan.steps.iter().enumerate() {
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Step(id) = input
                && let Some(slot) = last.get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
            {
                *slot = Some(index);
            }
        }
    }
    let emitted_step = usize::try_from(plan.emit.step.0).unwrap_or(usize::MAX);
    for (index, step) in plan.steps.iter().enumerate() {
        if step.scope == StepScope::PerFile {
            continue;
        }
        let (before, rest) = computed.split_at_mut(index);
        let out = rest.first_mut().ok_or(OperatingError::NotRun)?;
        // The host's rows are put in place as they are; no operator makes them.
        if let Operation::Hosted { relation } = &step.operation {
            let table = hosted
                .get(usize::try_from(relation.0).unwrap_or(usize::MAX))
                .ok_or(OperatingError::NotRun)?;
            copy_table(table, out);
            continue;
        }
        let inputs = inputs_of(&step.operation);
        let mut rows = std::array::from_fn::<_, 3, _>(|_| Rows::Computed(&NOTHING));
        for (slot, input) in rows.iter_mut().zip(named(&inputs)) {
            let handed_over = frontier
                .iter()
                .find(|each| each.input == *input)
                .map(|each| &each.table);
            *slot = match (input, handed_over) {
                (_, Some(table)) => Rows::Computed(table),
                (Input::Step(id), None) => match step_scope(plan, *id) {
                    Some(StepScope::CrossFile) => {
                        Rows::Computed(step_in(before, *id).ok_or(OperatingError::NotRun)?)
                    }
                    Some(StepScope::PerFile) | None => return Err(OperatingError::NotRun),
                },
                (Input::Relation(_), None) => return Ok(Table::default()),
            };
        }
        let used = rows.get(..inputs.used.0).unwrap_or_default();
        if let Some(slot) = fed.get_mut(index) {
            for (counted, each) in slot.0.iter_mut().zip(used) {
                *counted =
                    counted.saturating_add(u64::try_from(count_of(each).0).unwrap_or(u64::MAX));
            }
        }
        compute(&step.operation, used, context, out)?;
        // Rows made from the absence of rows, where rows of the relations could be missing, are dropped if the missing ones could change them.
        let uncertain = files_for(partials, needs.get(index).map_or(&[][..], Vec::as_slice));
        if !uncertain.is_empty() {
            let probed = match &step.operation {
                Operation::Antijoin { .. } | Operation::Difference { .. } => rows.get(1),
                Operation::Select { .. }
                | Operation::Project { .. }
                | Operation::Extend { .. }
                | Operation::Join { .. }
                | Operation::Union { .. }
                | Operation::Intersection { .. }
                | Operation::GroupCount { .. }
                | Operation::Replace { .. }
                | Operation::SpanBetween { .. }
                | Operation::OrderedMatch { .. }
                | Operation::OrderedDfs { .. }
                | Operation::GroupDigest { .. }
                | Operation::Capture { .. }
                | Operation::Rank { .. }
                | Operation::Words { .. }
                | Operation::Axis { .. }
                | Operation::Closure { .. }
                | Operation::Cycles { .. }
                | Operation::Rows { .. }
                | Operation::Hosted { .. } => rows.first(),
            };
            drop_uncertain(
                &step.operation,
                probed.map_or(&[][..], kinds_of),
                out,
                &uncertain,
            );
        }
        for (place, reader) in last.iter().enumerate() {
            if *reader == Some(index)
                && place != emitted_step
                && let Some(table) = computed.get_mut(place)
            {
                *table = Table::default();
            }
        }
    }
    match step_scope(plan, plan.emit.step) {
        Some(StepScope::CrossFile) => computed
            .into_iter()
            .nth(usize::try_from(plan.emit.step.0).unwrap_or(usize::MAX))
            .ok_or(OperatingError::NotRun),
        Some(StepScope::PerFile) | None => Err(OperatingError::NotRun),
    }
}
