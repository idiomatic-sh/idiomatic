//! The frontiers of the plans that carry named outputs: each per-file output a rule captures, and whether each frontier is an
//! observation's or the diagnostic's alone. Read from the plans alone, so every stage of the engine takes them from here.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::observation_schedule::types::{
    ObservedFrontierRoot, ResolvedObservationBinding, ResolvedObservationProducer,
};
use crate::domain::rule_intent::types::FrontierPurpose;
use ryo_values::domain::plan_parts::types::{Input, InputNumber, StepScope};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::ObservationIndex;
use std::collections::{HashMap, HashSet};

/// Each per-file named output a rule's frontier captures, in rule order, then output order: the producer, its output and the
/// frontier that carries it. An output no frontier carries, or one past what an index numbers, has none.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::observation_schedule::service::frontiers::service::observed_frontiers;
/// assert!(observed_frontiers(&[]).is_empty());
/// ```
#[must_use]
pub fn observed_frontiers(plans: &[Plan]) -> Vec<ObservedFrontierRoot> {
    let mut frontiers = Vec::new();
    for (rule, plan) in plans.iter().enumerate() {
        let positions: HashMap<Input, usize> = plan
            .frontier
            .iter()
            .copied()
            .enumerate()
            .map(|(at, input)| (input, at))
            .collect();
        for (output, root) in plan.observations.iter().enumerate() {
            if root.scope != StepScope::PerFile {
                continue;
            }
            if let (Some(&frontier), Ok(output)) =
                (positions.get(&root.source), u32::try_from(output))
            {
                frontiers.push(ObservedFrontierRoot {
                    producer: RuleIndex(rule),
                    output: ObservationIndex(output),
                    frontier: InputNumber(frontier),
                });
            }
        }
    }
    frontiers
}

/// Whether each frontier of each plan is read on the way to a named output (an observation's) or only the diagnostic's.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::observation_schedule::service::frontiers::service::frontier_purposes;
/// assert!(frontier_purposes(&[]).is_empty());
/// ```
#[must_use]
pub fn frontier_purposes(plans: &[Plan]) -> Vec<Vec<FrontierPurpose>> {
    plans
        .iter()
        .map(|plan| {
            let mut needed = HashSet::new();
            let mut pending = plan
                .observations
                .iter()
                .map(|output| output.source)
                .collect::<Vec<_>>();
            while let Some(input) = pending.pop() {
                if !needed.insert(input) {
                    continue;
                }
                if let Input::Step(step) = input
                    && let Some(step) = plan
                        .steps
                        .get(usize::try_from(step.0).unwrap_or(usize::MAX))
                {
                    pending.extend(named(&inputs_of(&step.operation)));
                }
            }
            plan.frontier
                .iter()
                .map(|input| {
                    if needed.contains(input) {
                        FrontierPurpose::Observation
                    } else {
                        FrontierPurpose::Diagnostic
                    }
                })
                .collect()
        })
        .collect()
}

/// The rules that consume each named output of each plan, in consumer order, each once, from the request's resolved bindings.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::observation_schedule::service::frontiers::service::output_consumers;
/// assert!(output_consumers(&[], &[]).is_empty());
/// ```
#[must_use]
pub fn output_consumers(
    plans: &[Plan],
    bindings: &[Vec<ResolvedObservationBinding>],
) -> Vec<Vec<Vec<RuleIndex>>> {
    let mut consumers: Vec<Vec<Vec<RuleIndex>>> = plans
        .iter()
        .map(|plan| vec![Vec::new(); plan.observations.len()])
        .collect();
    for (consumer, inputs) in bindings.iter().enumerate() {
        let consumer = RuleIndex(consumer);
        for producer in inputs.iter().flat_map(|binding| &binding.producers) {
            if let ResolvedObservationProducer::Output { rule, output } = producer
                && let Some(found) = consumers.get_mut(rule.0).and_then(|outputs| {
                    outputs.get_mut(usize::try_from(output.0).unwrap_or(usize::MAX))
                })
                && !found.contains(&consumer)
            {
                found.push(consumer);
            }
        }
    }
    consumers
}

/// The outputs each consumer retires, from the request's resolved bindings: the producers' outputs it reads, in the order its
/// bindings name them, each once.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::observation_schedule::service::frontiers::service::retirements;
/// assert!(retirements(&[]).is_empty());
/// ```
#[must_use]
pub fn retirements(
    bindings: &[Vec<ResolvedObservationBinding>],
) -> Vec<Vec<crate::domain::observation_schedule::types::ObservationRetirement>> {
    bindings
        .iter()
        .map(|inputs| {
            let mut retired: Vec<
                crate::domain::observation_schedule::types::ObservationRetirement,
            > = Vec::new();
            for producer in inputs.iter().flat_map(|binding| &binding.producers) {
                if let ResolvedObservationProducer::Output { rule, output } = producer
                    && !retired
                        .iter()
                        .any(|each| each.producer == *rule && each.output == *output)
                {
                    retired.push(
                        crate::domain::observation_schedule::types::ObservationRetirement {
                            producer: *rule,
                            output: *output,
                        },
                    );
                }
            }
            retired
        })
        .collect()
}
