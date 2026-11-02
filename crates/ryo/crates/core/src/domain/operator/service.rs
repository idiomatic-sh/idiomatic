//! Plan result delivery uses the mandatory application-owned operator engine.
pub(crate) use super::contract::registration::registered;
use super::types::{Pool, RowsFed};
use crate::domain::coverage::{service::drop_uncertain, types::Partials};
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operator::contract::{executor::PrimitiveExecutor, types::OperatorContract};
use crate::domain::table_model::{
    service::{copy_table, count_of, kinds_of},
    types::{Frontier, RowCount, Rows, Table},
};
use crate::domain::trust::service::{files_for, needs_of};
use ryo_values::domain::plan_parts::{service::step_in, types::Input};
use ryo_values::domain::plans::{
    service::{inputs_of, named, step_scope},
    types::{Operation, Plan, StepScope},
};
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
pub(crate) fn evaluate_cross_with_executor(
    plan: &Plan,
    frontier: Vec<Frontier>,
    partials: &Partials,
    context: &mut OperatorContext<'_>,
    fed: &mut [RowsFed],
    hosted: &[Table],
    executor: &dyn PrimitiveExecutor,
    operators: &[std::sync::Arc<OperatorContract>],
) -> Result<Table, OperatingError> {
    let needs = crate::domain::trust::service::needs_of_contracts(plan, operators)?;
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
        // Hosted is supplied as an explicit request source to its registered evaluator.
        let hosted_source = if let Operation::Hosted { relation } = &step.operation {
            Some(Rows::Computed(
                hosted
                    .get(relation.0 as usize)
                    .ok_or(OperatingError::NotRun)?,
            ))
        } else {
            None
        };
        let source_rows = hosted_source.into_iter().collect::<Vec<_>>();
        let used = if source_rows.is_empty() {
            rows.get(..inputs.used.0).unwrap_or_default()
        } else {
            &source_rows
        };
        if let Some(slot) = fed.get_mut(index) {
            for (counted, each) in slot.0.iter_mut().zip(used) {
                *counted =
                    counted.saturating_add(u64::try_from(count_of(each).0).unwrap_or(u64::MAX));
            }
        }
        executor.collect(
            operators.get(index).ok_or(OperatingError::NotRun)?,
            used,
            context,
            out,
        )?;
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

#[cfg(test)]
pub(crate) fn compute(
    operation: &Operation,
    inputs: &[Rows<'_>],
    context: &mut OperatorContext<'_>,
    out: &mut Table,
) -> Result<(), OperatingError> {
    let schemas = inputs
        .iter()
        .map(|rows| crate::domain::operation_schema::types::InputSchema {
            kinds: kinds_of(rows),
            tree: match rows {
                Rows::Tree(tree) => Some(tree.form),
                _ => None,
            },
        })
        .collect::<Vec<_>>();
    let contract = registered(crate::domain::features::service::kind_of(operation)).prepare(
        operation,
        &schemas,
        context.bindings,
    )?;
    crate::application::file_dag::execution::CommonOperatorEngine
        .collect(&contract, inputs, context, out)
}

#[cfg(test)]
pub(crate) fn evaluate_cross(
    plan: &Plan,
    frontier: Vec<Frontier>,
    partials: &Partials,
    context: &mut OperatorContext<'_>,
    fed: &mut [RowsFed],
    hosted: &[Table],
) -> Result<Table, OperatingError> {
    let bindings = vec![context.bindings.to_vec()];
    let request = crate::domain::operator::contract::types::RequestOperators::prepare(
        std::slice::from_ref(plan),
        &bindings,
        &[],
    )?;
    evaluate_cross_with_executor(
        plan,
        frontier,
        partials,
        context,
        fed,
        hosted,
        &crate::application::file_dag::execution::CommonOperatorEngine,
        &request.rules[0],
    )
}
