//! Request-local demand closure over immutable cached plans; no source names are resolved here.
use crate::domain::file_program::{
    errors::FileProgramError,
    types::{ResolvedObservationBinding, ResolvedObservationProducer},
};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::{
    plan_parts::types::{Input, StepId},
    plans::{
        service::{each_input, inputs_of, named},
        types::Plan,
    },
    records::types::ObservationIndex,
};

/// A linked request owns its remapped plans and typed producer references. Cached plans are untouched.
pub(crate) struct LinkedObservationPlans {
    pub plans: Vec<Plan>,
    pub bindings: Vec<Vec<ResolvedObservationBinding>>,
}

/// Follow only roots reachable from Emit or from actually read bound inputs, then remap once.
pub(crate) fn link(
    mut plans: Vec<Plan>,
    bindings: Vec<Vec<ResolvedObservationBinding>>,
) -> Result<LinkedObservationPlans, FileProgramError> {
    if bindings.iter().all(Vec::is_empty) && plans.iter().all(|plan| plan.observations.is_empty()) {
        return Ok(LinkedObservationPlans { plans, bindings });
    }
    let mut wanted: Vec<_> = plans
        .iter()
        .map(|plan| vec![false; plan.steps.len()])
        .collect();
    let mut exports: Vec<_> = plans
        .iter()
        .map(|plan| vec![false; plan.observations.len()])
        .collect();
    let mut used: Vec<_> = bindings
        .iter()
        .map(|inputs| vec![false; inputs.len()])
        .collect();
    let mut pending: Vec<_> = plans
        .iter()
        .enumerate()
        .map(|(rule, plan)| (rule, plan.emit.step))
        .collect();
    while let Some((rule, id)) = pending.pop() {
        let at = usize::try_from(id.0).map_err(|_| FileProgramError::InvalidReference)?;
        let seen = wanted
            .get_mut(rule)
            .and_then(|steps| steps.get_mut(at))
            .ok_or(FileProgramError::InvalidReference)?;
        if *seen {
            continue;
        }
        *seen = true;
        let step = plans
            .get(rule)
            .and_then(|plan| plan.steps.get(at))
            .ok_or(FileProgramError::InvalidReference)?;
        for input in named(&inputs_of(&step.operation)) {
            match *input {
                Input::Step(step) => pending.push((rule, step)),
                Input::Relation(_) => {}
                Input::BoundObservation(input) => {
                    let inputs = bindings
                        .get(rule)
                        .ok_or(FileProgramError::InvalidReference)?;
                    let position = inputs
                        .iter()
                        .position(|binding| binding.input == input)
                        .ok_or(FileProgramError::InvalidReference)?;
                    used.get_mut(rule)
                        .and_then(|inputs| inputs.get_mut(position))
                        .ok_or(FileProgramError::InvalidReference)
                        .map(|used| *used = true)?;
                    for producer in &inputs[position].producers {
                        if let ResolvedObservationProducer::Output {
                            rule: producer,
                            output,
                        } = producer
                        {
                            let output = usize::try_from(output.0)
                                .map_err(|_| FileProgramError::InvalidReference)?;
                            let observed = plans
                                .get(producer.0)
                                .and_then(|plan| plan.observations.get(output))
                                .ok_or(FileProgramError::InvalidReference)?;
                            let Input::Step(source) = observed.source else {
                                return Err(FileProgramError::InvalidReference);
                            };
                            exports
                                .get_mut(producer.0)
                                .and_then(|outputs| outputs.get_mut(output))
                                .ok_or(FileProgramError::InvalidReference)
                                .map(|used| *used = true)?;
                            pending.push((producer.0, source));
                        }
                    }
                }
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
                .and_then(|used| used.get(position))
                .copied()
                .ok_or(FileProgramError::InvalidReference)?;
            if !demanded {
                continue;
            }
            for producer in &mut binding.producers {
                if let ResolvedObservationProducer::Output { rule, output } = producer {
                    *output = output_maps
                        .get(rule.0)
                        .and_then(|mapping| mapping.get(usize::try_from(output.0).ok()?))
                        .and_then(|id| *id)
                        .ok_or(FileProgramError::InvalidReference)?;
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

/// A per-file producer needs an escape only when an actual cross-file step reads its bound input.
fn cross_file_exports(
    plans: &[Plan],
    bindings: &[Vec<ResolvedObservationBinding>],
    wanted: &[Vec<bool>],
) -> Result<Vec<Vec<bool>>, FileProgramError> {
    let mut escapes: Vec<_> = plans
        .iter()
        .map(|plan| vec![false; plan.observations.len()])
        .collect();
    for (rule, plan) in plans.iter().enumerate() {
        if plan.observation_inputs.is_empty() {
            continue;
        }
        let dependencies = super::observations::bound_step_dependencies(plan)?;
        for (at, step) in plan.steps.iter().enumerate() {
            if step.scope != StepScope::CrossFile || !wanted[rule][at] {
                continue;
            }
            for id in dependencies
                .get(at)
                .ok_or(FileProgramError::InvalidReference)?
            {
                let binding = bindings
                    .get(rule)
                    .and_then(|inputs| inputs.iter().find(|binding| binding.input == *id))
                    .ok_or(FileProgramError::InvalidReference)?;
                for producer in &binding.producers {
                    if let ResolvedObservationProducer::Output { rule, output } = producer {
                        let at = usize::try_from(output.0)
                            .map_err(|_| FileProgramError::InvalidReference)?;
                        let observed = plans
                            .get(rule.0)
                            .and_then(|plan| plan.observations.get(at))
                            .ok_or(FileProgramError::InvalidReference)?;
                        if observed.scope == StepScope::PerFile {
                            escapes[rule.0][at] = true;
                        }
                    }
                }
            }
        }
    }
    Ok(escapes)
}

/// Reindex all step references together, preserving original rule identities and parameter positions.
fn prune(
    plan: &mut Plan,
    wanted: &[bool],
    exports: &[bool],
    escapes: &[bool],
) -> Result<Vec<Option<ObservationIndex>>, FileProgramError> {
    let mut step_map = vec![None; plan.steps.len()];
    let mut next = 0_u32;
    for (at, keep) in wanted.iter().copied().enumerate() {
        if keep {
            step_map[at] = Some(StepId(next));
            next = next
                .checked_add(1)
                .ok_or(FileProgramError::TooManyOutputs)?;
        }
    }
    let remap = |id: StepId| {
        step_map
            .get(usize::try_from(id.0).map_err(|_| FileProgramError::InvalidReference)?)
            .copied()
            .flatten()
            .ok_or(FileProgramError::InvalidReference)
    };
    let mut steps = Vec::new();
    for (at, mut step) in std::mem::take(&mut plan.steps).into_iter().enumerate() {
        if !wanted[at] {
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
        if !exports[at] {
            continue;
        }
        let Input::Step(id) = observation.source else {
            return Err(FileProgramError::InvalidReference);
        };
        observation.source = Input::Step(remap(id)?);
        if escapes[at] {
            extra_frontier.push(observation.source);
        }
        output_map[at] = Some(ObservationIndex(
            u32::try_from(observations.len()).map_err(|_| FileProgramError::TooManyOutputs)?,
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

fn frontier(plan: &Plan) -> Result<Vec<Input>, FileProgramError> {
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
                        .get(
                            usize::try_from(id.0)
                                .map_err(|_| FileProgramError::InvalidReference)?,
                        )
                        .ok_or(FileProgramError::InvalidReference)?
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
