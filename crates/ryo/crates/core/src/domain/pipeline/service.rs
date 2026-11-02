//! The `pipeline` service: free functions named by verb that run the per-file steps of a plan on one file's rows, each step
//! passing its rows on as it makes them.

// ignore: idiomatic.arch/idiomatic-rust/module-cycles - the steps feed on the steps they run: the service runs a step through the `Feed` impl of `Stepper`, which reads its input by running the service's steps again (`run`, `operate`, `deliver`), one ring by design
use ryo_language::domain::relations::types::{Relation, Value};

use crate::domain::column_kernel::{service as column_kernel, types::KernelStrategy};
use crate::domain::fragment::service as lowered;
use crate::domain::fragment::types::FragmentRun;
use crate::domain::layout::service::layout_of;
use crate::domain::lowering::types::Counting;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, InputPlace, OperatorContext};
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
    let mut room = stepper
        .rooms
        .get_mut(id)
        .and_then(Option::take)
        .unwrap_or_default();
    let previous = std::mem::replace(&mut stepper.current, StepPlace(id));
    let executor = stepper.executor;
    let contract = std::sync::Arc::clone(stepper.operators.get(id).ok_or(OperatingError::NotRun)?);
    let outcome = executor.stream_step(&contract, stepper, StepPlace(id), context, &mut room, emit);
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

/// Borrow a graph output at its actual consumer, preserving opaque evaluation
/// order. OnceCell leases belong to this invocation and carry no mutable state.
fn dag_table<'a>(
    stepper: &Stepper<'a, '_>,
    id: usize,
    context: &mut OperatorContext<'_>,
) -> Result<Option<&'a Table>, OperatingError> {
    let Some(dag) = stepper.dag else {
        return Ok(None);
    };
    let Some(node) = dag.owned(stepper.rule, id) else {
        return Ok(None);
    };
    let leases = stepper.dag_shared;
    let slot = leases.get(id).ok_or(OperatingError::NotRun)?;
    if slot.get().is_none() {
        let table = dag.demand(node, &|relation| stepper.read.rows(relation), context)?;
        let _installed = slot.set(table);
    }
    Ok(slot.get().map(|table| table.as_ref()))
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
    if let Some(table) = dag_table(stepper, id.0, context)? {
        let mut buffer = stepper.spare.pop().unwrap_or_default();
        let outcome = scan(&Rows::Computed(table), &mut buffer, context, emit);
        stepper.spare.push(buffer);
        return outcome;
    }
    if stepper.layout.kept.0.get(id.0) == Some(&true) || stepper.graph_kept.get(id.0) == Some(&true)
    {
        let id = id.0;
        if let Some(table) = stepper.shared.get(id).and_then(Option::as_ref) {
            let mut buffer = stepper.spare.pop().unwrap_or_default();
            let outcome = scan(&Rows::Computed(table.as_ref()), &mut buffer, context, emit);
            stepper.spare.push(buffer);
            return outcome;
        }
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
            if let Some(table) = dag_table(stepper, id, context)? {
                return Ok(Rows::Computed(table));
            }

            if stepper.layout.kept.0.get(id) == Some(&true)
                || stepper.graph_kept.get(id) == Some(&true)
            {
                if let Some(table) = stepper.shared.get(id).and_then(Option::as_ref) {
                    return Ok(Rows::Computed(table.as_ref()));
                }
                let table = stepper.done.get(id).ok_or(OperatingError::NotRun)?;
                return CompactInput::new(stepper).kept(id, table);
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
#[cfg(test)]
pub(crate) fn evaluate_file<'f>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],
) -> Result<(), OperatingError> {
    evaluate_file_with_graph(
        plan, relations, tree, context, pool, prepared, classes, None, 0,
    )
}

/// Opt-in request graph execution; the ordinary entry point above keeps the
/// production strategy and its failure order unchanged.
/// Preserve ordinary eager roots, except pure ancestors already satisfied by a
/// retained successful cut. A dependency used elsewhere stays live. The caller
/// pins the retained tables until this rule completes, so publication cannot
/// evict the cut after this demand decision.
fn demanded_steps(plan: &Plan, eager: &[bool], cached: &[bool]) -> Vec<bool> {
    let dependencies = |index: usize| plan.steps.get(index).map(|step| inputs_of(&step.operation));
    let mut bypassed = vec![false; plan.steps.len()];
    let mut pending = Vec::new();
    for (index, retained) in cached.iter().enumerate() {
        if *retained && let Some(inputs) = dependencies(index) {
            pending.extend(named(&inputs).iter().copied());
        }
    }
    while let Some(input) = pending.pop() {
        let Input::Step(step) = input else { continue };
        let index = step.0 as usize;
        let Some(seen) = bypassed.get_mut(index) else {
            continue;
        };
        if *seen {
            continue;
        }
        *seen = true;
        if let Some(inputs) = dependencies(index) {
            pending.extend(named(&inputs).iter().copied());
        }
    }
    let mut demanded = vec![false; plan.steps.len()];
    pending.push(Input::Step(plan.emit.step));
    pending.extend(plan.frontier.iter().copied());
    pending.extend(eager.iter().enumerate().filter_map(|(index, eager)| {
        (*eager && !bypassed[index]).then_some(Input::Step(StepId(index as u32)))
    }));
    while let Some(input) = pending.pop() {
        let Input::Step(step) = input else { continue };
        let index = step.0 as usize;
        let Some(seen) = demanded.get_mut(index) else {
            continue;
        };
        if *seen {
            continue;
        }
        *seen = true;
        if cached.get(index) == Some(&true) {
            continue;
        }
        if let Some(inputs) = dependencies(index) {
            pending.extend(named(&inputs).iter().copied());
        }
    }
    demanded
}

#[cfg(test)]
pub(crate) fn evaluate_file_with_graph<'f>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],
    file_program: Option<&mut crate::domain::file_program::types::FileProgram>,
    rule: usize,
) -> Result<(), OperatingError> {
    evaluate_file_with_dag(
        plan,
        relations,
        tree,
        context,
        pool,
        prepared,
        classes,
        file_program,
        rule,
        None,
    )
}

#[cfg(test)]
pub(crate) fn evaluate_file_with_dag<'f>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],
    file_program: Option<&mut crate::domain::file_program::types::FileProgram>,
    rule: usize,
    file_dag: Option<&dyn crate::domain::file_program::types::DagExecution>,
) -> Result<(), OperatingError> {
    let request = crate::domain::operator::contract::types::RequestOperators::prepare_with_schemas(
        std::slice::from_ref(plan),
        &[context.bindings.to_vec()],
        &[],
        &|relation| {
            relations(relation).map_or_else(
                || ryo_language::domain::relations::service::kinds(relation),
                |view| view.kinds,
            )
        },
    )?;
    evaluate_file_with_executor(
        plan,
        relations,
        tree,
        context,
        pool,
        prepared,
        classes,
        file_program,
        rule,
        file_dag,
        &crate::application::file_dag::execution::CommonOperatorEngine,
        &request.rules[0],
    )
}

pub(crate) fn evaluate_file_with_executor<'f>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],
    mut file_program: Option<&mut crate::domain::file_program::types::FileProgram>,
    rule: usize,
    file_dag: Option<&dyn crate::domain::file_program::types::DagExecution>,
    executor: &dyn crate::domain::operator::contract::executor::PrimitiveExecutor,
    operators: &[std::sync::Arc<crate::domain::operator::contract::types::OperatorContract>],
) -> Result<(), OperatingError> {
    if let Some(dag) = file_dag {
        dag.begin_rule(rule);
    }
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
        let mut layout = layout_of(plan, identity, &read, context)?;
        layout.needs = crate::domain::trust::service::needs_of_contracts(plan, operators)?;
        if tracing::enabled!(target: "ryo::diagnostic::compiled_subplan", tracing::Level::TRACE) {
            tracing::event!(
                target: "ryo::diagnostic::compiled_subplan", tracing::Level::TRACE,
                record_kind = "layout", plan_identity = identity.0,
                kept = ?layout.kept.0, kinds = ?layout.kinds,
                lowering = ?layout.lowering, needs = ?layout.needs,
            );
        }
        prepared.layouts.push(layout);
        if prepared.kernel_strategy == KernelStrategy::Column {
            let kernel = column_kernel::compile(
                plan,
                prepared.layouts.last().ok_or(OperatingError::NotRun)?,
            );
            prepared.kernel_counts.compiled_chains +=
                kernel.0.iter().filter(|chain| chain.is_some()).count() as u64;
            prepared.kernel_plans.push(kernel);
        }
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
        kernel_strategy,
        kernel_plans,
        kernel_counts,
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
    let graph_owned = file_dag
        .is_some_and(|dag| (0..plan.steps.len()).any(|index| dag.owned(rule, index).is_some()));
    let lowered = match tree {
        _ if graph_owned => LoweredTree::Missing,
        Some(each) if *unlowered == TreeSteps::Lowered => LoweredTree::Tree(TreeRows {
            form: TreeForm::Elements,
            ..each
        }),
        _ => LoweredTree::Missing,
    };
    let cut_diagnostic =
        tracing::enabled!(target: "ryo::diagnostic::subplan_cut", tracing::Level::TRACE);
    let mut shared_steps = if file_program.is_some() || file_dag.is_some() {
        vec![None; plan.steps.len()]
    } else {
        Vec::new()
    };
    let mut graph_kept = if file_program.is_some() || file_dag.is_some() {
        vec![false; plan.steps.len()]
    } else {
        Vec::new()
    };
    // Pin existing complete successful cuts before pruning ancestors. These
    // handles use the existing retained buffers, not an additional row cache.
    let mut retained = Vec::new();
    let mut share_nodes = Vec::new();
    let mut eager = Vec::new();
    if let Some(program) = file_program.as_deref() {
        for (index, step) in plan.steps.iter().enumerate() {
            let lowered_cut = matches!(lowered, LoweredTree::Tree(_))
                && (layout
                    .lowering
                    .shapes
                    .get(index)
                    .is_some_and(Option::is_some)
                    || layout
                        .lowering
                        .counting
                        .get(index)
                        .copied()
                        .unwrap_or(Counting::DoesNotCount)
                        == Counting::Counts);
            // A formerly streamed step is made eagerly only for total typed
            // filtering/projection/union over complete inputs. Already kept joins
            // retain their original evaluation boundary and may also be shared.
            let force_safe = matches!(
                &step.operation,
                ryo_values::domain::plans::types::Operation::Select { .. }
                    | ryo_values::domain::plans::types::Operation::Project { .. }
                    | ryo_values::domain::plans::types::Operation::Union { .. }
            );
            let share_node = (!lowered_cut
                && (layout.kept.0.get(index) == Some(&true) || force_safe))
                .then(|| program.cut(rule, index))
                .flatten();
            let table = share_node.and_then(|node| {
                program
                    .entries
                    .iter()
                    .find(|entry| entry.0 == node)
                    .map(|entry| std::sync::Arc::clone(&entry.1))
            });
            retained.push(table);
            share_nodes.push(share_node);
            eager.push(
                step.scope == StepScope::PerFile
                    && (layout.kept.0.get(index) == Some(&true) || share_node.is_some()),
            );
        }
    }
    let demanded = retained.iter().any(Option::is_some).then(|| {
        let cached = retained.iter().map(Option::is_some).collect::<Vec<_>>();
        demanded_steps(plan, &eager, &cached)
    });
    let dag_shared = if file_dag.is_some() {
        (0..plan.steps.len())
            .map(|_| std::cell::OnceCell::new())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    // Only opaque steps remain in this schedule. Their Feed requests drive the
    // unique application DAG at the original lazy/error boundary.
    let execution_steps = file_dag.map(|dag| {
        (0..plan.steps.len())
            .filter(|index| dag.owned(rule, *index).is_none())
            .collect::<Vec<_>>()
    });
    let schedule = execution_steps.as_deref();
    for cursor in 0..schedule.map_or(plan.steps.len(), <[usize]>::len) {
        let index = schedule.map_or(cursor, |steps| steps[cursor]);
        let step = &plan.steps[index];
        if step.scope == StepScope::CrossFile {
            continue;
        }
        if demanded.as_ref().is_some_and(|steps| !steps[index]) {
            tracing::trace!(target: "ryo::diagnostic::file_program",
                event = "ancestor_pruned", rule, step = index);
            continue;
        }
        let share_node = share_nodes.get(index).copied().flatten();
        if layout.kept.0.get(index) != Some(&true) && share_node.is_none() {
            continue;
        }
        if let Some(slot) = graph_kept.get_mut(index) {
            *slot = share_node.is_some();
        }
        let (before, rest) = steps.split_at_mut(index);
        let target = rest.first_mut().ok_or(OperatingError::NotRun)?;
        if let Some(shared) = share_node.and_then(|node| file_program.as_deref_mut()?.get(node)) {
            if cut_diagnostic {
                diagnostic_cut(
                    identity.0,
                    tree.map(|tree| tree.file.0),
                    index,
                    &shared,
                    None,
                    made.ready.get(index),
                    partial,
                );
            }
            shared_steps[index] = Some(shared);
            continue;
        }
        reset(
            target,
            layout.kinds.get(index).ok_or(OperatingError::NotRun)?,
        );
        if layout
            .needs
            .get(index)
            .is_some_and(|needs| blocked(needs, partial) == Blocking::Blocked)
        {
            continue;
        }
        let mut stepper = Stepper {
            kernel_strategy: *kernel_strategy,
            kernel: kernel_plans.get(place),
            kernel_counts: &mut *kernel_counts,
            executor,
            operators,
            plan,
            read: &read,
            done: before,
            selected: &selected[..index],
            shared: shared_steps.get(..index).unwrap_or(&[]),
            graph_kept: graph_kept.get(..index).unwrap_or(&[]),
            dag: file_dag,
            dag_shared: &dag_shared,
            rule,
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
        let mut room = stepper
            .rooms
            .get_mut(index)
            .and_then(Option::take)
            .unwrap_or_default();
        let contract = std::sync::Arc::clone(operators.get(index).ok_or(OperatingError::NotRun)?);
        let materialization = executor.keep_step(
            &contract,
            &mut stepper,
            StepPlace(index),
            context,
            &mut room,
            target,
        );
        if let Some(slot) = stepper.rooms.get_mut(index) {
            *slot = Some(room);
        }
        let materialization = materialization?;
        let materialized = matches!(
            &materialization,
            crate::domain::operator::contract::executor::StepMaterialization::Materialized
        );
        let readiness = cut_diagnostic
            .then(|| stepper.made.ready.get(index).copied())
            .flatten();
        drop(stepper);
        if let crate::domain::operator::contract::executor::StepMaterialization::Selected(
            selection,
        ) = materialization
        {
            selected[index] = Some(selection);
        }
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
        if !materialized {
            continue;
        }
        if let Some(node) = share_node
            && let Some(shared) = file_program
                .as_deref_mut()
                .and_then(|program| program.publish(node, target))
        {
            shared_steps[index] = Some(shared);
        }
    }
    drop(retained);
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
