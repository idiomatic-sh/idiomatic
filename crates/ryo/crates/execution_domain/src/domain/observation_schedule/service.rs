//! The request-level observation schedule: the declared dependency edges between rules, the schemas their bound inputs
//! resolve to, and the plans linked beneath them. No providers or operators run here.

pub mod link;

use super::types::{
    PreparedObservationInputs, ResolvedObservationBinding, ResolvedObservationProducer,
    RuleSchedule,
};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::rule_intent::errors::RuleIntentError;
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
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::observation_schedule::{service::schedule, types::RuleSchedule};
/// assert_eq!(schedule(&[])?, RuleSchedule::Independent);
/// # Ok::<(), ryo_execution_domain::domain::rule_intent::errors::RuleIntentError>(())
/// ```
///
/// # Errors
///
/// The first [`RuleIntentError`] of the declared edges: a duplicate binding, an unknown producer, a self dependency,
/// a producer without a query, or a cycle.
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
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::observation_schedule::service::schemas;
/// use ryo_values::domain::plan_parts::types::QueryText;
/// use ryo_values::domain::plans::types::Plan;
/// use ryo_values::domain::query_tokens::types::QueryBundle;
/// use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
/// use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
/// let rule = RuleSpec {
///     id: RuleId("reads-nothing".into()),
///     query: QueryBundle::from(QueryText("".into())),
///     applicability: RuleApplicability::Always,
///     given: Vec::new(),
///     observations: Vec::new(),
///     spelled: RuleParameters("".into()),
///     include: Vec::new(),
///     exclude: Vec::new(),
/// };
/// let prepared = schemas(&rule, std::slice::from_ref(&rule), &[None::<Plan>])?;
/// assert!(prepared.schemas.is_empty());
/// assert!(prepared.bindings.is_empty());
/// # Ok::<(), ryo_execution_domain::domain::rule_intent::errors::RuleIntentError>(())
/// ```
///
/// # Errors
///
/// The first [`RuleIntentError`] of the consumer's bindings: an unknown or uncompiled producer, a missing output,
/// producers whose columns disagree, or more outputs than a `u32` numbers.
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
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::observation_schedule::service::request;
/// use ryo_execution_domain::domain::observation_schedule::types::{ObservationRequest, RuleSchedule};
/// use ryo_values::domain::records::types::ObservationReadinessLayout;
/// use ryo_values::domain::stored_facts::types::Digest;
/// let layout = ObservationReadinessLayout(Digest([0; 16]));
/// let independent = request(&RuleSchedule::Independent, &[], layout);
/// assert!(matches!(independent, Ok(ObservationRequest::Independent)));
/// ```
///
/// # Errors
///
/// [`RuleIntentError::InvalidReference`] when bindings are given to an independent schedule.
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
