//! Plan result delivery uses the mandatory application-owned operator engine.
use super::executor::PrimitiveExecutor;
use crate::domain::coverage::{service::drop_uncertain, types::Partials};
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operator::contract::types::OperatorContract;
use crate::domain::operator::types::RowsFed;
use crate::domain::table_model::{
    service::{copy_table, count_of, kinds_of},
    types::{Frontier, Rows},
};
use crate::domain::trust::service::{files_for, needs_of};
use ryo_values::domain::plan_parts::{service::step_in, types::Input};
use ryo_values::domain::plans::{
    service::{inputs_of, named, step_scope},
    types::{Operation, Plan, StepScope},
};

/// Whatever an operator refuses, or `NotRun`/`NotRead` when a step reads what nothing provides.
pub(crate) fn evaluate_cross_roots_with_executor(
    plan: &Plan,
    frontier: Vec<Frontier>,
    refused: &[crate::domain::crossing::types::RefusedFrontier],
    partials: &Partials,
    observation_partials: &Partials,
    context: &mut OperatorContext<'_>,
    fed: &mut [RowsFed],
    hosted: &[Table],
    executor: &dyn PrimitiveExecutor,
    operators: &[std::sync::Arc<OperatorContract>],
) -> Result<crate::domain::crossing::types::PhaseRoots, OperatingError> {
    let needs = crate::domain::trust::service::needs_of_contracts(plan, operators)?;
    let publication_reads = (!plan.observations.is_empty())
        .then(|| crate::domain::trust::service::physical_reads(plan));
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
    let observed_steps: Vec<_> = plan
        .observations
        .iter()
        .filter_map(|output| {
            if output.scope != StepScope::CrossFile {
                return None;
            }
            let Input::Step(step) = output.source else {
                return None;
            };
            usize::try_from(step.0).ok()
        })
        .collect();
    let mut failures: Option<Vec<Option<OperatingError>>> = (!plan.observations.is_empty()
        || !plan.observation_inputs.is_empty())
    .then(|| vec![None; plan.steps.len()]);
    for (index, step) in plan.steps.iter().enumerate() {
        if step.scope == StepScope::PerFile {
            continue;
        }
        #[cfg(feature = "diagnostic-row-progress")]
        let _row_progress_step = {
            let guard = super::route_counts::step(index);
            super::route_counts::node_description(&step.operation);
            guard
        };
        let result = (|| -> Result<(), OperatingError> {
            for input in named(&inputs_of(&step.operation)) {
                if let Some(fault) = refused.iter().find(|fault| fault.input == *input) {
                    return Err(fault.error.clone());
                }
                if let Input::Step(id) = input
                    && let Some(Some(error)) = failures
                        .as_ref()
                        .and_then(|failures| failures.get(usize::try_from(id.0).ok()?))
                {
                    return Err(error.clone());
                }
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
                    (Input::Relation(_), None) => return Err(OperatingError::NotRead),
                    (Input::BoundObservation(_), None) => return Err(OperatingError::NotRun),
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
            #[cfg(feature = "diagnostic-row-progress")]
            for (which, input) in used.iter().enumerate() {
                super::route_counts::input_cardinality(which, count_of(input).0);
            }
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
                    | Operation::RenderSyntax { .. }
                    | Operation::FitsRendered { .. }
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
            Ok(())
        })();
        if let Err(error) = result {
            if plan.observations.is_empty() && plan.observation_inputs.is_empty() {
                // Preserve the legacy absent-relation diagnostic path. Named outputs
                // never interpret an unobserved input as a complete empty publication.
                if error == OperatingError::NotRead
                    && named(&inputs_of(&step.operation)).iter().any(|input| {
                        matches!(input, Input::Relation(_))
                            && !frontier.iter().any(|entry| entry.input == *input)
                    })
                {
                    return Ok(crate::domain::crossing::types::PhaseRoots {
                        emit_error: None,
                        emit: Some(Table::default()),
                        observations: Vec::new(),
                    });
                }
                return Err(error);
            }
            if let Some(failures) = &mut failures {
                failures[index] = Some(error);
            }
        }
        for (place, reader) in last.iter().enumerate() {
            if *reader == Some(index)
                && place != emitted_step
                && !observed_steps.contains(&place)
                && let Some(table) = computed.get_mut(place)
            {
                *table = Table::default();
            }
        }
    }
    let emit_error = (step_scope(plan, plan.emit.step) == Some(StepScope::CrossFile))
        .then(|| {
            failures
                .as_ref()
                .and_then(|failures| failures.get(emitted_step))
                .and_then(Option::as_ref)
                .cloned()
        })
        .flatten();
    let emit = match step_scope(plan, plan.emit.step) {
        Some(StepScope::CrossFile) if emit_error.is_none() => Some(std::mem::take(
            computed
                .get_mut(emitted_step)
                .ok_or(OperatingError::NotRun)?,
        )),
        Some(StepScope::CrossFile | StepScope::PerFile) => None,
        None => return Err(OperatingError::NotRun),
    };
    let mut observations = Vec::with_capacity(observed_steps.len());
    for (index, observed) in plan.observations.iter().enumerate() {
        if observed.scope != StepScope::CrossFile {
            continue;
        }
        let Input::Step(step) = observed.source else {
            return Err(OperatingError::NotRun);
        };
        let at = usize::try_from(step.0).map_err(|_| OperatingError::NotRun)?;
        let root_reads = publication_reads
            .as_ref()
            .and_then(|reads| reads.get(at))
            .ok_or(OperatingError::NotRun)?;
        let uncertain = files_for(observation_partials, root_reads);
        let failure = failures
            .as_ref()
            .and_then(|failures| failures.get(at))
            .and_then(Option::as_ref);
        let complete = uncertain.is_empty() && failure.is_none();
        // Readiness is decided before copying any potentially large unpublishable table.
        let table = if complete {
            Some(if at == emitted_step {
                emit.as_ref().cloned().ok_or(OperatingError::NotRun)?
            } else {
                computed.get(at).cloned().ok_or(OperatingError::NotRun)?
            })
        } else {
            None
        };
        observations.push(crate::domain::crossing::types::ObservedRoot {
            output: ryo_values::domain::observations::types::ObservationIndex(
                u32::try_from(index).map_err(|_| OperatingError::NotRun)?,
            ),
            table,
            readiness: if complete {
                ryo_values::domain::observations::types::ProducerReadiness::Complete
            } else {
                ryo_values::domain::observations::types::ProducerReadiness::Unknown(
                    ryo_values::domain::observations::types::ObservationReason(
                        failure.map_or_else(
                            || "producer dependencies are incomplete".into(),
                            |error| error.to_string().into(),
                        ),
                    ),
                )
            },
        });
    }
    Ok(crate::domain::crossing::types::PhaseRoots {
        emit,
        emit_error,
        observations,
    })
}

/// Legacy diagnostic-only caller of the same execution pass. A per-file Emit is not a cross result.
#[cfg(test)]
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
    let roots = evaluate_cross_roots_with_executor(
        plan,
        frontier,
        &[],
        partials,
        partials,
        context,
        fed,
        hosted,
        executor,
        operators,
    )?;
    if let Some(error) = roots.emit_error {
        return Err(error);
    }
    roots.emit.ok_or(OperatingError::NotRun)
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
    let contract = super::derived::prepare::operator(
        crate::domain::operator::contract::registration::registered(
            crate::domain::features::service::kind_of(operation),
        ),
        operation,
        &schemas,
        context.bindings,
    )?;
    crate::application::file_dag::execution::CommonOperatorEngine::fixture()
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
        crate::outbound::derived::factory::test_port(),
    )?;
    evaluate_cross_with_executor(
        plan,
        frontier,
        partials,
        context,
        fed,
        hosted,
        &crate::application::file_dag::execution::CommonOperatorEngine::fixture(),
        &request.rules[0],
    )
}

/// What a step that reads nothing reads.
static NOTHING: Table = Table {
    kinds: Vec::new(),
    cells: Vec::new(),
    count: RowCount(0),
};

use ryo_values::domain::rows::types::{RowCount, Table};
