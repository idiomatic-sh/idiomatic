//! Pure named-root dependency resolution; the existing application graph owns execution and resources.
use crate::domain::coverage::types::RuleIndex;
use crate::domain::file_program::constants::{EXPECT_BINDING_INDEX, EXPECT_OUTPUT_INDEX};
use crate::domain::file_program::types::{
    PreparedObservationInputs, ResolvedObservationBinding, ResolvedObservationProducer,
    RuleSchedule,
};
use crate::domain::rule_intent::errors::RuleIntentError;
use crate::domain::rule_intent::types::FrontierPurpose;
use ryo_values::domain::observations::types::{BoundObservationInput, ObservationProducer};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Plan;
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

/// Bind every output to the existing graph's actual node identity, with explicit unsupported admission.
pub(crate) fn graph_outputs(
    plans: &[Plan],
    nodes: &[Vec<Option<crate::domain::file_program::types::ProgramNodeId>>],
    request: crate::domain::file_program::types::ObservationRequest<'_>,
) -> Option<crate::domain::file_program::types::GraphObservations> {
    use crate::domain::file_program::types::{
        GraphObservationOutput, GraphObservationRoot, GraphObservations, ObservationRequest,
    };
    let ObservationRequest::Bound {
        bindings, layout, ..
    } = request
    else {
        return None;
    };
    let slots = plans
        .iter()
        .enumerate()
        .flat_map(|(rule, plan)| {
            plan.observation_inputs
                .iter()
                .enumerate()
                .map(
                    move |(input, _)| crate::domain::file_program::types::ObservationSlot {
                        rule: RuleIndex(rule),
                        input: ryo_values::domain::plan_parts::types::ObservationBindingId(
                            u32::try_from(input).expect(EXPECT_BINDING_INDEX),
                        ),
                    },
                )
        })
        .collect();
    let mut outputs: Vec<Vec<GraphObservationOutput>> = plans
        .iter()
        .enumerate()
        .map(|(rule, plan)| {
            plan.observations
                .iter()
                .map(|output| {
                    let root = if output.scope == StepScope::CrossFile {
                        GraphObservationRoot::CrossFile
                    } else {
                        match output.source {
                            ryo_values::domain::plan_parts::types::Input::Step(step) => nodes
                                .get(rule)
                                .and_then(|nodes| nodes.get(step.0 as usize))
                                .copied()
                                .flatten()
                                .map_or(
                                    GraphObservationRoot::Unavailable,
                                    GraphObservationRoot::PerFile,
                                ),
                            _ => GraphObservationRoot::Unavailable,
                        }
                    };
                    GraphObservationOutput {
                        root,
                        consumers: Vec::new(),
                    }
                })
                .collect()
        })
        .collect();
    let mut frontiers = Vec::new();
    for (rule, plan) in plans.iter().enumerate() {
        let positions: std::collections::HashMap<_, _> = plan
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
            if let Some(&frontier) = positions.get(&root.source) {
                frontiers.push(crate::domain::file_program::types::ObservedFrontierRoot {
                    producer: RuleIndex(rule),
                    output: ryo_values::domain::records::types::ObservationIndex(
                        u32::try_from(output).expect(EXPECT_OUTPUT_INDEX),
                    ),
                    frontier: ryo_values::domain::plan_parts::types::InputNumber(frontier),
                });
            }
        }
    }
    let mut retire = vec![Vec::new(); bindings.len()];
    for (consumer, inputs) in bindings.iter().enumerate() {
        let consumer = RuleIndex(consumer);
        for binding in inputs {
            for producer in &binding.producers {
                if let ResolvedObservationProducer::Output { rule, output } = producer
                    && let Some(root) = outputs
                        .get_mut(rule.0)
                        .and_then(|outputs| outputs.get_mut(output.0 as usize))
                    && !root.consumers.contains(&consumer)
                {
                    root.consumers.push(consumer);
                    retire[consumer.0].push(
                        crate::domain::file_program::types::ObservationRetirement {
                            producer: *rule,
                            output: *output,
                        },
                    );
                }
            }
        }
    }
    let bindings = bindings
        .iter()
        .map(|inputs| {
            inputs
                .iter()
                .map(|input| {
                    let kinds = input
                        .schema
                        .columns
                        .iter()
                        .map(|column| column.kind)
                        .collect::<Vec<_>>();
                    let schema = crate::domain::operation_schema::types::InputShape {
                        kinds: &kinds,
                        tree: None,
                    };
                    let union = ryo_values::domain::plans::types::Operation::Union {
                        left: ryo_values::domain::plan_parts::types::Input::Step(
                            ryo_values::domain::plan_parts::types::StepId(0),
                        ),
                        right: ryo_values::domain::plan_parts::types::Input::Step(
                            ryo_values::domain::plan_parts::types::StepId(1),
                        ),
                    };
                    crate::domain::file_program::types::GraphObservationBinding {
                        input: input.clone(),
                        union: crate::domain::operator::contract::service::prepare_for(
                            ryo_values::domain::query_operators::types::OperationKind::Union,
                            &union,
                            &[schema, schema],
                            &[],
                        )
                        .map(std::sync::Arc::new),
                    }
                })
                .collect()
        })
        .collect();
    let observation_frontiers = plans
        .iter()
        .map(|plan| {
            let mut needed = std::collections::HashSet::new();
            let mut pending = plan
                .observations
                .iter()
                .map(|output| output.source)
                .collect::<Vec<_>>();
            while let Some(input) = pending.pop() {
                if !needed.insert(input) {
                    continue;
                }
                if let ryo_values::domain::plan_parts::types::Input::Step(step) = input {
                    if let Some(step) = plan.steps.get(step.0 as usize) {
                        pending.extend(ryo_values::domain::plans::service::named(
                            &ryo_values::domain::plans::service::inputs_of(&step.operation),
                        ));
                    }
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
        .collect();
    Some(GraphObservations {
        observation_frontiers,
        layout,
        slots,
        frontiers,
        retire,
        bindings,
        outputs,
    })
}

/// A bound request cannot silently fall back to independent execution.
pub fn request<'a>(
    schedule: &'a RuleSchedule,
    bindings: &'a [Vec<ResolvedObservationBinding>],
    layout: ryo_values::domain::records::types::ObservationReadinessLayout,
) -> Result<crate::domain::file_program::types::ObservationRequest<'a>, RuleIntentError> {
    use crate::domain::file_program::types::ObservationRequest;
    if bindings.is_empty() {
        return Ok(ObservationRequest::Independent);
    }
    match schedule {
        RuleSchedule::Dependencies { .. } => Ok(ObservationRequest::Bound { layout, bindings }),
        RuleSchedule::Independent => Err(RuleIntentError::InvalidReference),
    }
}
