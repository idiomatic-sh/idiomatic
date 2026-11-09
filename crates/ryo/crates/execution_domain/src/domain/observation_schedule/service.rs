//! The request-level observation schedule: the declared dependency edges between rules, the schemas their bound inputs
//! resolve to, and the plans linked beneath them. No providers or operators run here.

use super::types::{
    BindingCount, EscapingOutputs, ExportedOutputs, LinkedObservationPlans, PendingStep,
    PreparedObservationInputs, ResolvedObservationBinding, ResolvedObservationProducer,
    RuleSchedule, UsedInputs, WantedSteps,
};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::rule_intent::errors::RuleIntentError;
use crate::domain::rule_intent::types::FrontierPurpose;
use ryo_values::domain::observations::types::{BoundObservationInput, ObservationProducer};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{Input, ObservationBindingId, StepId};
use ryo_values::domain::plans::service::{each_input, inputs_of, named};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::RuleId;
use std::collections::{BTreeMap, BTreeSet};

/// Resolve only declared dependency edges. No providers or operators are executed here.
pub fn schedule(rules: &[RuleSpec]) -> Result<RuleSchedule, RuleIntentError> {
    if rules.iter().all(|rule| rule.observations.is_empty()) {
        return Ok(RuleSchedule::Independent);
    }
    let index: BTreeMap<&RuleId, usize> = rules
        .iter()
        .enumerate()
        .map(|(at, rule)| (&rule.id, at))
        .collect();
    let mut needs = vec![BTreeSet::new(); rules.len()];
    for (consumer, rule) in rules.iter().enumerate() {
        let mut names = BTreeSet::new();
        for binding in &rule.observations {
            if !names.insert(&binding.name) {
                return Err(RuleIntentError::DuplicateBinding {
                    rule: rule.id.0.to_string(),
                    name: binding.name.0.to_string(),
                });
            }
            for producer in &binding.producers {
                match producer {
                    ObservationProducer::Output { rule: id, .. } => {
                        let Some(&producer) = index.get(id) else {
                            return Err(RuleIntentError::UnknownProducer {
                                consumer: rule.id.0.to_string(),
                                producer: id.0.to_string(),
                            });
                        };
                        if producer == consumer {
                            return Err(RuleIntentError::SelfDependency {
                                rule: id.0.to_string(),
                            });
                        }
                        needs[consumer].insert(producer);
                    }
                    ObservationProducer::NoQuery { rule: id } if index.contains_key(id) => {
                        return Err(RuleIntentError::ExecutableNoQuery {
                            rule: id.0.to_string(),
                        });
                    }
                    ObservationProducer::NoQuery { .. } => {}
                }
            }
        }
    }
    let mut done = vec![false; rules.len()];
    let mut order = Vec::with_capacity(rules.len());
    let mut batches = Vec::new();
    while order.len() < rules.len() {
        let ready: Vec<_> = needs
            .iter()
            .enumerate()
            .filter(|(at, reads)| !done[*at] && reads.iter().all(|producer| done[*producer]))
            .map(|(at, _)| RuleIndex(at))
            .collect();
        if ready.is_empty() {
            return Err(RuleIntentError::Cycle);
        }
        for rule in &ready {
            done[rule.0] = true;
        }
        order.extend(ready.iter().copied());
        batches.push(ready);
    }
    Ok(RuleSchedule::Dependencies { order, batches })
}

/// Infer actual schemas/phases after earlier producer plans compiled, without renumbering rule IDs.
pub fn schemas<P: std::borrow::Borrow<Plan>>(
    consumer: &RuleSpec,
    rules: &[RuleSpec],
    compiled: &[Option<P>],
) -> Result<PreparedObservationInputs, RuleIntentError> {
    let mut schemas = Vec::new();
    let mut bindings = Vec::new();
    for (local, binding) in consumer.observations.iter().enumerate() {
        let mut scope = if binding.producers.is_empty() {
            StepScope::CrossFile
        } else {
            StepScope::PerFile
        };
        let mut producers = Vec::new();
        for reference in &binding.producers {
            let ObservationProducer::Output { rule, output } = reference else {
                if let ObservationProducer::NoQuery { rule } = reference {
                    producers.push(ResolvedObservationProducer::NoQuery { rule: rule.clone() });
                }
                scope = StepScope::CrossFile;
                continue;
            };
            let at = rules
                .iter()
                .position(|candidate| candidate.id == *rule)
                .ok_or_else(|| RuleIntentError::UnknownProducer {
                    consumer: consumer.id.0.to_string(),
                    producer: rule.0.to_string(),
                })?;
            let plan = compiled.get(at).and_then(Option::as_ref).ok_or_else(|| {
                RuleIntentError::NotCompiled {
                    rule: rule.0.to_string(),
                }
            })?;
            let plan: &Plan = std::borrow::Borrow::borrow(plan);
            let (output_index, observed) = plan
                .observations
                .iter()
                .enumerate()
                .find(|(_, observed)| observed.name == *output)
                .ok_or_else(|| RuleIntentError::MissingOutput {
                    producer: rule.0.to_string(),
                    output: output.0.to_string(),
                })?;
            if observed.columns != binding.columns {
                return Err(RuleIntentError::IncompatibleObservationColumns {
                    consumer: consumer.id.0.to_string(),
                    producer: rule.0.to_string(),
                    output: output.0.to_string(),
                });
            }
            if observed.scope == StepScope::CrossFile {
                scope = StepScope::CrossFile;
            }
            let output_index =
                u32::try_from(output_index).map_err(|_| RuleIntentError::TooManyOutputs)?;
            producers.push(ResolvedObservationProducer::Output {
                rule: RuleIndex(at),
                output: ryo_values::domain::records::types::ObservationIndex(output_index),
            });
        }
        let schema = BoundObservationInput {
            name: binding.name.clone(),
            columns: binding.columns.clone(),
            scope,
        };
        let local = u32::try_from(local).map_err(|_| RuleIntentError::TooManyOutputs)?;
        bindings.push(ResolvedObservationBinding {
            input: ryo_values::domain::plan_parts::types::ObservationBindingId(local),
            schema: schema.clone(),
            producers,
        });
        schemas.push(schema);
    }
    Ok(PreparedObservationInputs { schemas, bindings })
}

/// A bound request cannot silently fall back to independent execution.
pub fn request<'a>(
    schedule: &'a RuleSchedule,
    bindings: &'a [Vec<ResolvedObservationBinding>],
    layout: ryo_values::domain::records::types::ObservationReadinessLayout,
) -> Result<super::types::ObservationRequest<'a>, RuleIntentError> {
    use super::types::ObservationRequest;
    if bindings.is_empty() {
        return Ok(ObservationRequest::Independent);
    }
    match schedule {
        RuleSchedule::Dependencies { .. } => Ok(ObservationRequest::Bound { layout, bindings }),
        RuleSchedule::Independent => Err(RuleIntentError::InvalidReference),
    }
}

/// Follow only roots reachable from Emit or from actually read bound inputs, then remap once.
pub fn link(
    mut plans: Vec<Plan>,
    bindings: Vec<Vec<ResolvedObservationBinding>>,
) -> Result<LinkedObservationPlans, RuleIntentError> {
    if bindings.iter().all(Vec::is_empty) && plans.iter().all(|plan| plan.observations.is_empty()) {
        return Ok(LinkedObservationPlans { plans, bindings });
    }
    let mut wanted: Vec<_> = plans
        .iter()
        .map(|plan| WantedSteps(vec![false; plan.steps.len()]))
        .collect();
    let mut exports: Vec<_> = plans
        .iter()
        .map(|plan| ExportedOutputs(vec![false; plan.observations.len()]))
        .collect();
    let mut used: Vec<_> = bindings
        .iter()
        .map(|inputs| UsedInputs(vec![false; inputs.len()]))
        .collect();
    let mut pending: Vec<_> = plans
        .iter()
        .enumerate()
        .map(|(rule, plan)| PendingStep {
            rule: RuleIndex(rule),
            step: plan.emit.step,
        })
        .collect();
    while let Some(PendingStep { rule, step: id }) = pending.pop() {
        let at = usize::try_from(id.0).map_err(|_| RuleIntentError::InvalidReference)?;
        let seen = wanted
            .get_mut(rule.0)
            .and_then(|steps| steps.0.get_mut(at))
            .ok_or(RuleIntentError::InvalidReference)?;
        if *seen {
            continue;
        }
        *seen = true;
        let step = plans
            .get(rule.0)
            .and_then(|plan| plan.steps.get(at))
            .ok_or(RuleIntentError::InvalidReference)?;
        for input in named(&inputs_of(&step.operation)) {
            match *input {
                Input::Step(step) => pending.push(PendingStep { rule, step }),
                Input::Relation(_) => {}
                Input::BoundObservation(input) => demand_bound_input(
                    rule,
                    input,
                    &plans,
                    &bindings,
                    &mut used,
                    &mut exports,
                    &mut pending,
                )?,
            }
        }
    }
    let escapes = cross_file_exports(&plans, &bindings, &wanted)?;
    let mut output_maps = Vec::with_capacity(plans.len());
    for (rule, plan) in plans.iter_mut().enumerate() {
        output_maps.push(prune(plan, &wanted[rule], &exports[rule], &escapes[rule])?);
    }
    let mut kept_bindings = Vec::new();
    for (rule, inputs) in bindings.into_iter().enumerate() {
        let mut kept = Vec::new();
        for (position, mut binding) in inputs.into_iter().enumerate() {
            let demanded = used
                .get(rule)
                .and_then(|used| used.0.get(position))
                .copied()
                .ok_or(RuleIntentError::InvalidReference)?;
            if !demanded {
                continue;
            }
            for producer in &mut binding.producers {
                if let ResolvedObservationProducer::Output { rule, output } = producer {
                    let Ok(at) = usize::try_from(output.0) else {
                        return Err(RuleIntentError::InvalidReference);
                    };
                    *output = output_maps
                        .get(rule.0)
                        .and_then(|mapping| mapping.get(at))
                        .and_then(|id| *id)
                        .ok_or(RuleIntentError::InvalidReference)?;
                }
            }
            kept.push(binding);
        }
        kept_bindings.push(kept);
    }
    Ok(LinkedObservationPlans {
        plans,
        bindings: kept_bindings,
    })
}

/// Demand the producers of one bound input that a kept step reads, and the roots they export.
fn demand_bound_input(
    rule: RuleIndex,
    input: ObservationBindingId,
    plans: &[Plan],
    bindings: &[Vec<ResolvedObservationBinding>],
    used: &mut [UsedInputs],
    exports: &mut [ExportedOutputs],
    pending: &mut Vec<PendingStep>,
) -> Result<(), RuleIntentError> {
    let inputs = bindings
        .get(rule.0)
        .ok_or(RuleIntentError::InvalidReference)?;
    let position = inputs
        .iter()
        .position(|binding| binding.input == input)
        .ok_or(RuleIntentError::InvalidReference)?;
    used.get_mut(rule.0)
        .and_then(|inputs| inputs.0.get_mut(position))
        .ok_or(RuleIntentError::InvalidReference)
        .map(|used| *used = true)?;
    for producer in &inputs[position].producers {
        let ResolvedObservationProducer::Output {
            rule: owner,
            output,
        } = producer
        else {
            continue;
        };
        let output = usize::try_from(output.0).map_err(|_| RuleIntentError::InvalidReference)?;
        let observed = plans
            .get(owner.0)
            .and_then(|plan| plan.observations.get(output))
            .ok_or(RuleIntentError::InvalidReference)?;
        let Input::Step(source) = observed.source else {
            return Err(RuleIntentError::InvalidReference);
        };
        exports
            .get_mut(owner.0)
            .and_then(|outputs| outputs.0.get_mut(output))
            .ok_or(RuleIntentError::InvalidReference)
            .map(|used| *used = true)?;
        pending.push(PendingStep {
            rule: *owner,
            step: source,
        });
    }
    Ok(())
}

/// A per-file producer needs an escape only when an actual cross-file step reads its bound input.
fn cross_file_exports(
    plans: &[Plan],
    bindings: &[Vec<ResolvedObservationBinding>],
    wanted: &[WantedSteps],
) -> Result<Vec<EscapingOutputs>, RuleIntentError> {
    let mut escapes: Vec<_> = plans
        .iter()
        .map(|plan| EscapingOutputs(vec![false; plan.observations.len()]))
        .collect();
    for (rule, plan) in plans.iter().enumerate() {
        if plan.observation_inputs.is_empty() {
            continue;
        }
        let dependencies = bound_step_dependencies(plan)?;
        for (at, step) in plan.steps.iter().enumerate() {
            if step.scope != StepScope::CrossFile || !wanted[rule].0[at] {
                continue;
            }
            for id in dependencies
                .get(at)
                .ok_or(RuleIntentError::InvalidReference)?
            {
                let binding = bindings
                    .get(rule)
                    .and_then(|inputs| inputs.iter().find(|binding| binding.input == *id))
                    .ok_or(RuleIntentError::InvalidReference)?;
                mark_escapes(plans, binding, &mut escapes)?;
            }
        }
    }
    Ok(escapes)
}

/// Mark the per-file outputs that one bound input reads for a cross-file step.
fn mark_escapes(
    plans: &[Plan],
    binding: &ResolvedObservationBinding,
    escapes: &mut [EscapingOutputs],
) -> Result<(), RuleIntentError> {
    for producer in &binding.producers {
        let ResolvedObservationProducer::Output { rule, output } = producer else {
            continue;
        };
        let at = usize::try_from(output.0).map_err(|_| RuleIntentError::InvalidReference)?;
        let observed = plans
            .get(rule.0)
            .and_then(|plan| plan.observations.get(at))
            .ok_or(RuleIntentError::InvalidReference)?;
        if observed.scope == StepScope::PerFile {
            escapes[rule.0].0[at] = true;
        }
    }
    Ok(())
}

/// Reindex all step references together, preserving original rule identities and parameter positions.
fn prune(
    plan: &mut Plan,
    wanted: &WantedSteps,
    exports: &ExportedOutputs,
    escapes: &EscapingOutputs,
) -> Result<Vec<Option<ObservationIndex>>, RuleIntentError> {
    let mut step_map = vec![None; plan.steps.len()];
    let mut next = StepId(0);
    for (at, keep) in wanted.0.iter().copied().enumerate() {
        if keep {
            step_map[at] = Some(next);
            next = following(next)?;
        }
    }
    let remap = |id: StepId| {
        step_map
            .get(usize::try_from(id.0).map_err(|_| RuleIntentError::InvalidReference)?)
            .copied()
            .flatten()
            .ok_or(RuleIntentError::InvalidReference)
    };
    let mut steps = Vec::new();
    for (at, mut step) in std::mem::take(&mut plan.steps).into_iter().enumerate() {
        if !wanted.0[at] {
            continue;
        }
        let mut failure = None;
        each_input(&mut step.operation, &mut |input| {
            if let Input::Step(id) = input {
                match remap(*id) {
                    Ok(mapped) => *id = mapped,
                    Err(error) => failure = Some(error),
                }
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        steps.push(step);
    }
    plan.emit.step = remap(plan.emit.step)?;
    let mut output_map = vec![None; plan.observations.len()];
    let mut observations = Vec::new();
    let mut extra_frontier = Vec::new();
    for (at, mut observation) in std::mem::take(&mut plan.observations)
        .into_iter()
        .enumerate()
    {
        if !exports.0[at] {
            continue;
        }
        let Input::Step(id) = observation.source else {
            return Err(RuleIntentError::InvalidReference);
        };
        observation.source = Input::Step(remap(id)?);
        if escapes.0[at] {
            extra_frontier.push(observation.source);
        }
        output_map[at] = Some(ObservationIndex(
            u32::try_from(observations.len()).map_err(|_| RuleIntentError::TooManyOutputs)?,
        ));
        observations.push(observation);
    }
    plan.steps = steps;
    plan.observations = observations;
    plan.frontier = frontier(plan)?;
    for input in extra_frontier {
        if !plan.frontier.contains(&input) {
            plan.frontier.push(input);
        }
    }
    Ok(output_map)
}

/// The step number after `id`; a plan cannot have more steps than numbers.
pub(crate) fn following(id: StepId) -> Result<StepId, RuleIntentError> {
    id.0.checked_add(1)
        .map(StepId)
        .ok_or(RuleIntentError::TooManyOutputs)
}

fn frontier(plan: &Plan) -> Result<Vec<Input>, RuleIntentError> {
    let mut frontier = Vec::new();
    for step in plan
        .steps
        .iter()
        .filter(|step| step.scope == StepScope::CrossFile)
    {
        for input in named(&inputs_of(&step.operation)) {
            let outside = match input {
                Input::Relation(_) | Input::BoundObservation(_) => true,
                Input::Step(id) => {
                    plan.steps
                        .get(usize::try_from(id.0).map_err(|_| RuleIntentError::InvalidReference)?)
                        .ok_or(RuleIntentError::InvalidReference)?
                        .scope
                        == StepScope::PerFile
                }
            };
            if outside && !frontier.contains(input) {
                frontier.push(*input);
            }
        }
    }
    Ok(frontier)
}

/// Actual bound sources beneath each retained cross frontier, including per-file transforms.
pub fn frontier_dependencies(
    plan: &Plan,
) -> Result<Vec<Vec<ObservationBindingId>>, RuleIntentError> {
    if plan.observation_inputs.is_empty() {
        return Ok(vec![Vec::new(); plan.frontier.len()]);
    }
    let steps = bound_step_dependencies(plan)?;
    plan.frontier
        .iter()
        .map(|input| {
            bound_input_dependencies(*input, &steps, BindingCount(plan.observation_inputs.len()))
                .map(|set| set.into_iter().collect())
        })
        .collect()
}
/// One topological pass over the existing plan, used for escape and frontier admission preparation.
pub(crate) fn bound_step_dependencies(
    plan: &Plan,
) -> Result<
    Vec<BTreeSet<ryo_values::domain::plan_parts::types::ObservationBindingId>>,
    RuleIntentError,
> {
    let mut steps = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let mut dependencies = BTreeSet::new();
        for input in ryo_values::domain::plans::service::named(
            &ryo_values::domain::plans::service::inputs_of(&step.operation),
        ) {
            dependencies.extend(bound_input_dependencies(
                *input,
                &steps,
                BindingCount(plan.observation_inputs.len()),
            )?);
        }
        steps.push(dependencies);
    }
    Ok(steps)
}
fn bound_input_dependencies(
    input: ryo_values::domain::plan_parts::types::Input,
    steps: &[BTreeSet<ryo_values::domain::plan_parts::types::ObservationBindingId>],
    count: BindingCount,
) -> Result<BTreeSet<ObservationBindingId>, RuleIntentError> {
    use ryo_values::domain::plan_parts::types::Input;
    match input {
        Input::Relation(_) => Ok(BTreeSet::new()),
        Input::Step(step) => steps
            .get(step.0 as usize)
            .cloned()
            .ok_or(RuleIntentError::InvalidReference),
        Input::BoundObservation(id) if (id.0 as usize) < count.0 => Ok(BTreeSet::from([id])),
        Input::BoundObservation(_) => Err(RuleIntentError::InvalidReference),
    }
}
