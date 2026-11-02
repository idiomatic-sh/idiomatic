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
use super::collections::operators::source_matches::item::SourceMatchesItem;
use super::collections::operators::span_between::item::SpanBetweenItem;
use super::collections::operators::span_bounds::item::SpanBoundsItem;
use super::collections::operators::union::item::UnionItem;
use super::collections::operators::words::item::WordsItem;
use super::collections::operators::{
    ordered_dfs::item::OrderedDfsItem, ordered_match::item::OrderedMatchItem,
    replace::item::ReplaceItem,
};
use super::types::{AbsencePolicy, Pool, RegisteredOperator, RowsFed};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, OperatorContext, Room};
use crate::domain::operation_schema::service::{
    join_schema, project_schema, select_schema, source_matches_schema, span_bounds_schema,
    union_schema,
};

use crate::domain::coverage::service::drop_uncertain;
use crate::domain::coverage::types::Partials;
use crate::domain::table_model::service::{copy_table, count_of, kinds_of};
use crate::domain::table_model::types::{Frontier, RowCount, Rows, Table};
use crate::domain::trust::service::{files_for, needs_of};
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::types::{Input, InputCount, Inputs};
use ryo_values::domain::plans::service::inputs_of;
use ryo_values::domain::plans::service::named;
use ryo_values::domain::plans::service::step_scope;
use ryo_values::domain::plans::types::{Operation, OperationKind, Plan, StepScope};

/// One executable pilot registry. Other variants retain exhaustive legacy
/// dispatch until they have equivalent metadata and conformance fixtures.
static PROJECT_OPERATOR: RegisteredOperator = RegisteredOperator {
    kind: OperationKind::Project,
    item: &ProjectItem,
    schema: project_schema,
    input: project_input,
    absence: AbsencePolicy::PositiveRows,
};
static SELECT_OPERATOR: RegisteredOperator = RegisteredOperator {
    kind: OperationKind::Select,
    item: &SelectItem,
    schema: select_schema,
    input: select_input,
    absence: AbsencePolicy::PositiveRows,
};
static JOIN_OPERATOR: RegisteredOperator = RegisteredOperator {
    kind: OperationKind::Join,
    item: &JoinItem,
    schema: join_schema,
    input: join_input,
    absence: AbsencePolicy::PositiveRows,
};
static UNION_OPERATOR: RegisteredOperator = RegisteredOperator {
    kind: OperationKind::Union,
    item: &UnionItem,
    schema: union_schema,
    input: union_input,
    absence: AbsencePolicy::PositiveRows,
};
static SOURCE_MATCHES_OPERATOR: RegisteredOperator = RegisteredOperator {
    kind: OperationKind::SourceMatches,
    item: &SourceMatchesItem,
    schema: source_matches_schema,
    input: source_matches_input,
    absence: AbsencePolicy::PositiveRows,
};
static SPAN_BOUNDS_OPERATOR: RegisteredOperator = RegisteredOperator {
    kind: OperationKind::SpanBounds,
    item: &SpanBoundsItem,
    schema: span_bounds_schema,
    input: span_bounds_input,
    absence: AbsencePolicy::PositiveRows,
};

fn one_input(from: Input) -> Inputs {
    Inputs {
        list: [from; 3],
        used: InputCount(1),
    }
}

fn two_inputs(left: Input, right: Input) -> Inputs {
    Inputs {
        list: [left, right, left],
        used: InputCount(2),
    }
}

fn select_input(operation: &Operation) -> Result<Inputs, OperatingError> {
    let Operation::Select { from, .. } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(one_input(*from))
}

fn project_input(operation: &Operation) -> Result<Inputs, OperatingError> {
    let Operation::Project { from, .. } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(one_input(*from))
}

fn join_input(operation: &Operation) -> Result<Inputs, OperatingError> {
    let Operation::Join { left, right, .. } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(two_inputs(*left, *right))
}

fn union_input(operation: &Operation) -> Result<Inputs, OperatingError> {
    let Operation::Union { left, right } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(two_inputs(*left, *right))
}

fn source_matches_input(operation: &Operation) -> Result<Inputs, OperatingError> {
    let Operation::SourceMatches { from, .. } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(one_input(*from))
}

fn span_bounds_input(operation: &Operation) -> Result<Inputs, OperatingError> {
    let Operation::SpanBounds { from, .. } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(one_input(*from))
}

/// The current registered subset only. An unregistered variant stays on its
/// existing exhaustive dispatch and cannot silently acquire shared policy.
pub(crate) fn pilot(operation: &Operation) -> Option<&'static RegisteredOperator> {
    match operation {
        Operation::Select { .. } => Some(&SELECT_OPERATOR),
        Operation::Project { .. } => Some(&PROJECT_OPERATOR),
        Operation::Join { .. } => Some(&JOIN_OPERATOR),
        Operation::Union { .. } => Some(&UNION_OPERATOR),
        Operation::SourceMatches { .. } => Some(&SOURCE_MATCHES_OPERATOR),
        Operation::SpanBounds { .. } => Some(&SPAN_BOUNDS_OPERATOR),
        _ => None,
    }
}

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
        Operation::Select { .. } => pilot(operation)
            .expect("registered Select")
            .item
            .stream(operation, ins, context, room, emit),
        Operation::Project { .. } => pilot(operation)
            .expect("registered Project")
            .item
            .stream(operation, ins, context, room, emit),
        Operation::Extend { .. } => ExtendItem.stream(operation, ins, context, room, emit),
        Operation::Join { .. } => pilot(operation)
            .expect("registered Join")
            .item
            .stream(operation, ins, context, room, emit),
        Operation::Union { .. } => pilot(operation)
            .expect("registered Union")
            .item
            .stream(operation, ins, context, room, emit),
        Operation::Intersection { .. } => {
            IntersectionItem.stream(operation, ins, context, room, emit)
        }
        Operation::Difference { .. } => DifferenceItem.stream(operation, ins, context, room, emit),
        Operation::Antijoin { .. } => AntijoinItem.stream(operation, ins, context, room, emit),
        Operation::Replace { .. } => ReplaceItem.stream(operation, ins, context, room, emit),
        Operation::SpanBetween { .. } => {
            SpanBetweenItem.stream(operation, ins, context, room, emit)
        }
        Operation::SourceMatches { .. } => pilot(operation)
            .expect("registered SourceMatches")
            .item
            .stream(operation, ins, context, room, emit),
        Operation::SpanBounds { .. } => pilot(operation)
            .expect("registered SpanBounds")
            .item
            .stream(operation, ins, context, room, emit),
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
        Operation::Select { .. } => pilot(operation)
            .expect("registered Select")
            .item
            .compute(operation, inputs, context, out),
        Operation::Project { .. } => pilot(operation)
            .expect("registered Project")
            .item
            .compute(operation, inputs, context, out),
        Operation::Extend { .. } => ExtendItem.compute(operation, inputs, context, out),
        Operation::Join { .. } => pilot(operation)
            .expect("registered Join")
            .item
            .compute(operation, inputs, context, out),
        Operation::Union { .. } => pilot(operation)
            .expect("registered Union")
            .item
            .compute(operation, inputs, context, out),
        Operation::Intersection { .. } => IntersectionItem.compute(operation, inputs, context, out),
        Operation::Difference { .. } => DifferenceItem.compute(operation, inputs, context, out),
        Operation::Antijoin { .. } => AntijoinItem.compute(operation, inputs, context, out),
        Operation::Replace { .. } => ReplaceItem.compute(operation, inputs, context, out),
        Operation::SpanBetween { .. } => SpanBetweenItem.compute(operation, inputs, context, out),
        Operation::SourceMatches { .. } => pilot(operation)
            .expect("registered SourceMatches")
            .item
            .compute(operation, inputs, context, out),
        Operation::SpanBounds { .. } => pilot(operation)
            .expect("registered SpanBounds")
            .item
            .compute(operation, inputs, context, out),
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

/// Mutable buffers sized for the supplied plans. Production uses one plan in a local rule workspace.
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
        ..Pool::default()
    }
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
                | Operation::SourceMatches { .. }
                | Operation::SpanBounds { .. }
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
