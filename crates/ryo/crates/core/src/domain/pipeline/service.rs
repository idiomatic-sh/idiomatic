//! The `pipeline` service: free functions named by verb that run the per-file steps of a plan on one file's rows, each step
//! passing its rows on as it makes them.

// ignore: idiomatic.arch/idiomatic-rust/module-cycles - the steps feed on the steps they run: the service runs a step through the `Feed` impl of `Stepper`, which reads its input by running the service's steps again (`run`, `operate`, `deliver`), one ring by design
use ryo_language::domain::relations::types::{Relation, Value};

use crate::domain::fragment::service as lowered;
use crate::domain::fragment::types::FragmentRun;
use crate::domain::layout::service::layout_of;
use crate::domain::lowering::types::Counting;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, InputPlace, OperatorContext};
use crate::domain::operator::service::operate;
use crate::domain::operator::types::{
    PlanIdentity, Pool, PreparedPool, RowsFed, StepPlace, TreeSteps,
};
use crate::domain::pipeline::types::compact::value::CompactInput;
use crate::domain::pipeline::types::{LoweredTree, Stepper};
use crate::domain::positions::types::Readiness;
use crate::domain::replay::service::scan;
use crate::domain::replay::types::{Collect, Pipe};
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::service::{copy_table, materialize, reset};
use crate::domain::table_model::types::{RowCount, Rows, Table};
use crate::domain::trust::service::blocked;
use crate::domain::trust::types::Blocking;
use ryo_language::domain::relations::types::ColumnsView;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Plan, StepScope};
use std::sync::atomic::{AtomicU64, Ordering};

/// Supplies distinct IDs to nested diagnostic step invocations.
static NEXT_DIAGNOSTIC_STEP_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// Diagnostic-only call ancestry; no executor state or rows are retained here.
    static DIAGNOSTIC_STEP_STACK: std::cell::RefCell<Vec<u64>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Restore the diagnostic ancestry even if an observed step unwinds.
struct DiagnosticStepGuard;

impl Drop for DiagnosticStepGuard {
    fn drop(&mut self) {
        DIAGNOSTIC_STEP_STACK.with(|stack| {
            let _completed = stack.borrow_mut().pop();
        });
    }
}

/// Observe one completed kept cut; a compact row-ID view and materialized cells are distinct
/// representations. These counts describe this invocation, not a proposed shared cache size.
fn diagnostic_cut(
    plan_identity: usize,
    file_id: Option<u32>,
    step_index: usize,
    table: &Table,
    selected: Option<&std::sync::Arc<crate::domain::table_model::types::StoredSelection>>,
    readiness: Option<&Readiness>,
    partial: &[crate::domain::coverage::types::Partial],
) {
    let compact_rows = selected.map_or(0, |selection| selection.rows.len());
    let compact_capacity = selected.map_or(0, |selection| selection.rows.capacity());
    let compact_identity =
        selected.map(|selection| std::sync::Arc::as_ptr(&selection.rows) as usize);
    tracing::event!(
        target: "ryo::diagnostic::subplan_cut", tracing::Level::TRACE,
        record_kind = "completed_cut", plan_identity, file_id = ?file_id, step_index,
        rows = table.count.0, cells = table.cells.len(), cell_capacity = table.cells.capacity(),
        compact_rows, compact_capacity, compact_identity = ?compact_identity,
        readiness = ?readiness, partial = ?partial,
    );
}

/// Notes that the step `consumer` was fed `rows` rows.
pub(crate) fn feed(
    stepper: &mut Stepper<'_, '_>,
    consumer: StepPlace,
    which: InputPlace,
    rows: RowCount,
) {
    let rows = u64::try_from(rows.0).unwrap_or(u64::MAX);
    if let Some(slot) = stepper
        .fed
        .get_mut(consumer.0)
        .and_then(|fed| fed.0.get_mut(which.0))
    {
        *slot = slot.saturating_add(rows);
    }
}

/// The input `which` of the current step.
///
/// # Errors
///
/// `MissingInput` when the step has no such input.
pub(crate) fn input(stepper: &Stepper<'_, '_>, which: InputPlace) -> Result<Input, OperatingError> {
    let step = stepper
        .plan
        .steps
        .get(stepper.current.0)
        .ok_or(OperatingError::NotRun)?;
    let inputs = inputs_of(&step.operation);
    named(&inputs)
        .get(which.0)
        .copied()
        .ok_or(OperatingError::MissingInput {
            position: which.0 + 1,
        })
}

/// Makes the rows of the step `id` and passes them on, whether or not it keeps them.
///
/// # Errors
///
/// Whatever the step, its inputs or `emit` refuse.
pub(crate) fn pass_on<'a, 'f>(
    stepper: &mut Stepper<'a, 'f>,
    id: StepPlace,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError>
where
    'f: 'a,
{
    let diagnostic = tracing::enabled!(
        target: "ryo::diagnostic::rule_step",
        tracing::Level::TRACE
    );
    if !diagnostic {
        return pass_on_inner(stepper, id, context, emit);
    }
    let operation = stepper
        .plan
        .steps
        .get(id.0)
        .map_or("missing", |step| operation_name(&step.operation));
    let details = stepper.plan.steps.get(id.0).map(|step| &step.operation);
    let plan_identity = std::ptr::from_ref(stepper.plan).addr();
    let parent_step_index = stepper.current.0;
    let invocation_id = NEXT_DIAGNOSTIC_STEP_ID.fetch_add(1, Ordering::Relaxed);
    let parent_invocation_id = DIAGNOSTIC_STEP_STACK.with(|stack| stack.borrow().last().copied());
    let step_span = tracing::trace_span!(
        target: "ryo::diagnostic::rule_step",
        "rule_step",
        invocation_id,
        parent_invocation_id = ?parent_invocation_id,
        plan_identity,
        step_index = id.0,
        parent_step_index,
        nested = parent_step_index != id.0,
        operation,
        operation_details = ?details,
    );
    let _entered = step_span.enter();
    let started = std::time::Instant::now();
    DIAGNOSTIC_STEP_STACK.with(|stack| stack.borrow_mut().push(invocation_id));
    let guard = DiagnosticStepGuard;
    tracing::trace!(
        target: "ryo::diagnostic::rule_step",
        event = "rule_step_start",
        invocation_id,
        parent_invocation_id = ?parent_invocation_id,
        plan_identity,
        step_index = id.0,
        parent_step_index,
        nested = parent_step_index != id.0,
        operation,
    );
    let outcome = pass_on_inner(stepper, id, context, emit);
    drop(guard);
    let completed_parent_step_index = stepper.current.0;
    tracing::trace!(
        target: "ryo::diagnostic::rule_step",
        event = "rule_step_end",
        invocation_id,
        parent_invocation_id = ?parent_invocation_id,
        plan_identity,
        step_index = id.0,
        parent_step_index = completed_parent_step_index,
        nested = completed_parent_step_index != id.0,
        operation,
        elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        outcome = if outcome.is_ok() { "ok" } else { "error" },
    );
    outcome
}

/// Executes one step and forwards its rows to the receiver.
fn pass_on_inner<'a, 'f>(
    stepper: &mut Stepper<'a, 'f>,
    id: StepPlace,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError>
where
    'f: 'a,
{
    let id = id.0;
    let plan = stepper.plan;
    // A step that answers from the absence of rows cannot where the rows could be missing: it makes none.
    if stepper
        .layout
        .needs
        .get(id)
        .is_some_and(|needs| blocked(needs, stepper.partial) == Blocking::Blocked)
    {
        return Ok(());
    }
    if let LoweredTree::Tree(tree) = stepper.lowered
        && (stepper
            .layout
            .lowering
            .shapes
            .get(id)
            .is_some_and(Option::is_some)
            || stepper
                .layout
                .lowering
                .counting
                .get(id)
                .copied()
                .unwrap_or(Counting::DoesNotCount)
                == Counting::Counts)
    {
        let mut run = FragmentRun {
            plan,
            lowering: &stepper.layout.lowering,
            tree,
            room: &mut *stepper.made,
            classes: stepper.classes,
        };
        let step = StepId(u32::try_from(id).unwrap_or(u32::MAX));
        if stepper.counting == Counting::DoesNotCount {
            if lowered::counts(&run, step) == Counting::Counts {
                return lowered::emit_counts(&mut run, step, context, emit);
            }
            lowered::make(&mut run, step, context)?;
            return lowered::emit(&mut run, step, context, emit);
        }
        // A step over the tree is fed the rows it makes: it reads the tree in place.
        let mut made = 0_usize;
        let outcome = {
            let mut counting = Pipe(|piped: &mut OperatorContext<'_>, row: &[Value]| {
                made += 1;
                emit.row(piped, row)
            });
            if lowered::counts(&run, step) == Counting::Counts {
                lowered::emit_counts(&mut run, step, context, &mut counting)
            } else {
                lowered::make(&mut run, step, context)
                    .and_then(|()| lowered::emit(&mut run, step, context, &mut counting))
            }
        };
        feed(stepper, StepPlace(id), InputPlace(0), RowCount(made));
        return outcome;
    }
    let operation = &plan.steps.get(id).ok_or(OperatingError::NotRun)?.operation;
    if stepper.counting == Counting::DoesNotCount
        && let Some(rows) = CompactInput::new(stepper).selected(StepPlace(id), context)?
    {
        let mut buffer = stepper.spare.pop().unwrap_or_default();
        let result = scan(&rows, &mut buffer, context, emit);
        stepper.spare.push(buffer);
        return result;
    }
    let mut room = stepper
        .rooms
        .get_mut(id)
        .and_then(Option::take)
        .unwrap_or_default();
    let previous = std::mem::replace(&mut stepper.current, StepPlace(id));
    let outcome = operate(operation, stepper, context, &mut room, emit);
    stepper.current = previous;
    if let Some(slot) = stepper.rooms.get_mut(id) {
        *slot = Some(room);
    }
    outcome
}

/// Returns the stable diagnostic label for an operation.
fn operation_name(operation: &ryo_values::domain::plans::types::Operation) -> &'static str {
    use ryo_values::domain::plans::types::Operation;
    match operation {
        Operation::Select { .. } => "select",
        Operation::Project { .. } => "project",
        Operation::Extend { .. } => "extend",
        Operation::Join { .. } => "join",
        Operation::Union { .. } => "union",
        Operation::Intersection { .. } => "intersection",
        Operation::Difference { .. } => "difference",
        Operation::Antijoin { .. } => "antijoin",
        Operation::GroupCount { .. } => "group-count",
        Operation::GroupDigest { .. } => "group-digest",
        Operation::Capture { .. } => "capture",
        Operation::Rank { .. } => "rank",
        Operation::Replace { .. } => "replace",
        Operation::OrderedMatch { .. } => "ordered-match",
        Operation::Words { .. } => "words",
        Operation::Axis { .. } => "axis",
        Operation::Closure { .. } => "closure",
        Operation::OrderedDfs { .. } => "ordered-dfs",
        Operation::Cycles { .. } => "cycles",
        Operation::Rows { .. } => "rows",
        Operation::Hosted { .. } => "hosted",
        Operation::SpanBetween { .. } => "span-between",
        Operation::SourceMatches { .. } => "source-matches",
        Operation::SpanBounds { .. } => "span-bounds",
    }
}

/// Passes on the rows of the step `id`: its table when it keeps them, else as it makes them.
///
/// # Errors
///
/// `NotRun` when a step that keeps its rows has not run, and whatever the step or `emit` refuse.
pub(crate) fn run<'a, 'f>(
    stepper: &mut Stepper<'a, 'f>,
    id: StepPlace,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError>
where
    'f: 'a,
{
    if stepper.layout.kept.0.get(id.0) == Some(&true) {
        let id = id.0;
        let table = stepper.done.get(id).ok_or(OperatingError::NotRun)?;
        let rows = CompactInput::new(stepper).kept(id, table)?;
        let mut buffer = stepper.spare.pop().unwrap_or_default();
        let outcome = scan(&rows, &mut buffer, context, emit);
        stepper.spare.push(buffer);
        outcome
    } else {
        pass_on(stepper, id, context, emit)
    }
}

/// Passes the rows of the input `which` to `emit`.
///
/// # Errors
///
/// Whatever the input or `emit` refuse.
pub(crate) fn deliver<'a, 'f>(
    stepper: &mut Stepper<'a, 'f>,
    which: InputPlace,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError>
where
    'f: 'a,
{
    match input(stepper, which)? {
        Input::Relation(relation) => {
            let rows = stepper.read.rows(relation)?;
            let mut buffer = stepper.spare.pop().unwrap_or_default();
            let outcome = scan(&rows, &mut buffer, context, emit);
            stepper.spare.push(buffer);
            outcome
        }
        Input::Step(id) => run(
            stepper,
            StepPlace(usize::try_from(id.0).unwrap_or(usize::MAX)),
            context,
            emit,
        ),
    }
}

/// The rows of the input `which`, whole.
///
/// # Errors
///
/// Whatever the input refuses.
pub(crate) fn keep<'a, 'f, 't>(
    stepper: &mut Stepper<'a, 'f>,
    which: InputPlace,
    context: &mut OperatorContext<'_>,
    into: &'t mut Table,
) -> Result<Rows<'t>, OperatingError>
where
    'a: 't,
    'f: 'a,
{
    match input(stepper, which)? {
        Input::Relation(relation) => Ok(stepper.read.rows(relation)?),
        Input::Step(id) => {
            let id = usize::try_from(id.0).unwrap_or(usize::MAX);
            if stepper.layout.kept.0.get(id) == Some(&true) {
                let table = stepper.done.get(id).ok_or(OperatingError::NotRun)?;
                return CompactInput::new(stepper).kept(id, table);
            }
            if stepper.counting == Counting::DoesNotCount
                && let Some(rows) = CompactInput::new(stepper).selected(StepPlace(id), context)?
            {
                return Ok(rows);
            }
            let kinds = stepper.layout.kinds.get(id).ok_or(OperatingError::NotRun)?;
            reset(into, kinds);
            pass_on(stepper, StepPlace(id), context, &mut Collect(&mut *into))?;
            Ok(Rows::Computed(into))
        }
    }
}

/// Runs the per-file steps of a plan on one file's rows into the pool: the rows of the steps that keep them, and of the inputs
/// handed to the cross-file steps. `relations` gives the rows of each relation the plan reads, of this file alone, where the store
/// keeps them. A step passes its rows on as it makes them.
///
/// # Errors
///
/// Whatever an operator refuses, or `NotRun`/`NotRead` when a step reads what nothing provides.
pub(crate) fn evaluate_file<'f>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],
) -> Result<(), OperatingError> {
    let read = |relation: Relation| -> Result<Rows<'f>, OperatingError> {
        match (relation, tree) {
            (Relation::SyntaxChild, Some(tree)) => Ok(Rows::Tree(TreeRows {
                form: TreeForm::Edges,
                ..tree
            })),
            (Relation::SyntaxTree, Some(tree)) => Ok(Rows::Tree(TreeRows {
                form: TreeForm::Elements,
                ..tree
            })),
            _ => relations(relation)
                .map(Rows::Stored)
                .ok_or(OperatingError::NotRead),
        }
    };
    let identity = PlanIdentity(std::ptr::from_ref(plan).addr());
    let place = if let Some(place) = prepared
        .layouts
        .iter()
        .position(|each| each.plan == identity)
    {
        place
    } else {
        let layout = layout_of(plan, identity, &read, context)?;
        if tracing::enabled!(target: "ryo::diagnostic::compiled_subplan", tracing::Level::TRACE) {
            tracing::event!(
                target: "ryo::diagnostic::compiled_subplan", tracing::Level::TRACE,
                record_kind = "layout", plan_identity = identity.0,
                kept = ?layout.kept.0, kinds = ?layout.kinds,
                lowering = ?layout.lowering, needs = ?layout.needs,
            );
        }
        prepared.layouts.push(layout);
        prepared.layouts.len() - 1
    };
    if prepared.fed.len() <= place {
        prepared.fed.resize_with(place + 1, Vec::new);
    }
    if pool.rooms.len() < plan.steps.len() {
        pool.rooms.resize_with(plan.steps.len(), || None);
    }
    if pool.selected.len() < plan.steps.len() {
        pool.selected.resize_with(plan.steps.len(), || None);
    }
    pool.selected.iter_mut().for_each(|slot| *slot = None);
    let PreparedPool {
        selection_plans,
        layouts,
        fed,
        unlowered,
        counting,
        ..
    } = prepared;
    let selectors = selection_plans
        .iter()
        .find(|prepared| prepared.identity.0 == identity.0)
        .map_or(&[][..], |prepared| prepared.steps.as_slice());
    let Pool {
        steps,
        selected,
        frontier,
        rooms,
        spare,
        made,
        partial,
        ..
    } = pool;
    made.ready.clear();
    made.ready.resize(plan.steps.len(), Readiness::NotMade);
    if made.made.len() < plan.steps.len() {
        made.made.resize_with(plan.steps.len(), Default::default);
    }
    let layout = layouts.get(place).ok_or(OperatingError::NotRun)?;
    let plan_fed = fed.get_mut(place).ok_or(OperatingError::NotRun)?;
    if plan_fed.len() < plan.steps.len() {
        plan_fed.resize(plan.steps.len(), RowsFed::default());
    }
    let lowered = match tree {
        Some(each) if *unlowered == TreeSteps::Lowered => LoweredTree::Tree(TreeRows {
            form: TreeForm::Elements,
            ..each
        }),
        _ => LoweredTree::Missing,
    };
    let cut_diagnostic =
        tracing::enabled!(target: "ryo::diagnostic::subplan_cut", tracing::Level::TRACE);
    for (index, step) in plan.steps.iter().enumerate() {
        if step.scope == StepScope::CrossFile || layout.kept.0.get(index) != Some(&true) {
            continue;
        }
        let (before, rest) = steps.split_at_mut(index);
        let target = rest.first_mut().ok_or(OperatingError::NotRun)?;
        reset(
            target,
            layout.kinds.get(index).ok_or(OperatingError::NotRun)?,
        );
        let mut stepper = Stepper {
            plan,
            read: &read,
            done: before,
            selected: &selected[..index],
            selectors,
            rooms,
            spare,
            layout,
            lowered,
            made,
            classes,
            current: StepPlace(index),
            partial,
            fed: &mut *plan_fed,
            counting: *counting,
        };
        if *counting == Counting::DoesNotCount
            && let Some(Rows::Selected(rows)) =
                CompactInput::new(&stepper).selected(StepPlace(index), context)?
        {
            let readiness = cut_diagnostic
                .then(|| stepper.made.ready.get(index).copied())
                .flatten();
            selected[index] = Some(rows.selection);
            if cut_diagnostic {
                diagnostic_cut(
                    identity.0,
                    tree.map(|tree| tree.file.0),
                    index,
                    target,
                    selected[index].as_ref(),
                    readiness.as_ref(),
                    partial,
                );
            }
            continue;
        }
        pass_on(
            &mut stepper,
            StepPlace(index),
            context,
            &mut Collect(target),
        )?;
        if cut_diagnostic {
            diagnostic_cut(
                identity.0,
                tree.map(|tree| tree.file.0),
                index,
                target,
                selected[index].as_ref(),
                stepper.made.ready.get(index),
                partial,
            );
        }
    }
    hand_over(plan, steps, selected, &read, context, frontier)?;
    // The exported/rendered table is the materialization boundary; intermediate kept steps stay compact.
    if let Some(selection) = usize::try_from(plan.emit.step.0)
        .ok()
        .and_then(|index| selected.get(index))
        .and_then(Option::as_ref)
    {
        let Rows::Stored(source) = read(selection.relation)? else {
            return Err(OperatingError::WrongOperation);
        };
        if let Some(target) = steps.get_mut(plan.emit.step.0 as usize) {
            materialize(
                &Rows::Selected(crate::domain::table_model::types::SelectedRows {
                    source,
                    selection: std::sync::Arc::clone(selection),
                }),
                target,
                &mut context.scratch.row,
            );
        }
    }
    Ok(())
}

/// The inputs the cross-file steps are handed, into `frontier`: a relation's rows of the file, or a per-file step's table.
///
/// # Errors
///
/// `NotRead` when a relation is not read, `NotRun` when a step has no table.
fn hand_over<'f>(
    plan: &Plan,
    steps: &[Table],
    selected: &[Option<std::sync::Arc<crate::domain::table_model::types::StoredSelection>>],
    read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    context: &mut OperatorContext<'_>,
    frontier: &mut [Table],
) -> Result<(), OperatingError> {
    for (input, into) in plan.frontier.iter().zip(frontier.iter_mut()) {
        match input {
            Input::Relation(relation) => {
                materialize(&read(*relation)?, into, &mut context.scratch.row);
            }
            Input::Step(id) => {
                if let Some(selection) = selected.get(id.0 as usize).and_then(Option::as_ref) {
                    let Rows::Stored(source) = read(selection.relation)? else {
                        return Err(OperatingError::WrongOperation);
                    };
                    materialize(
                        &Rows::Selected(crate::domain::table_model::types::SelectedRows {
                            source,
                            selection: std::sync::Arc::clone(selection),
                        }),
                        into,
                        &mut context.scratch.row,
                    );
                    continue;
                }
                let table = steps
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                    .ok_or(OperatingError::NotRun)?;
                copy_table(table, into);
            }
        }
    }
    Ok(())
}
