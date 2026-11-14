//! The `identity` application operation.

use super::super::constants::OBSERVATION_PRODUCERS_DOMAIN;
use super::super::errors::RuleCompilationError;
use ryo_execution_domain::domain::observation_schedule::types::{
    ResolvedObservationBinding, ResolvedObservationProducer, RuleSchedule,
};
use ryo_execution_domain::domain::rule_intent::errors::RuleIntentError;
use ryo_execution_runtime::outbound::digest::{interface::DigestPort, types::DigestBytes};
use ryo_values::domain::stores::types::{Crossing, StoredRule};

/// Extend existing per-rule identities in dependency order. Ordinary requests allocate nothing here.
pub(crate) fn with_producers(
    digest: &dyn DigestPort,
    mut rules: Vec<StoredRule>,
    plans: &[std::sync::Arc<ryo_values::domain::plans::types::Plan>],
    schedule: &RuleSchedule,
    bindings: &[Vec<ResolvedObservationBinding>],
) -> Result<super::super::types::PreparedRuleIdentities, RuleCompilationError> {
    let RuleSchedule::Dependencies { order, .. } = schedule else {
        return Ok(super::super::types::PreparedRuleIdentities {
            rules,
            observations: ryo_execution_domain::domain::prepared_request::types::ObservationIdentity::Independent,
        });
    };
    if rules.len() != plans.len() {
        return Err(RuleIntentError::InvalidReference.into());
    }
    let layout = rules
        .iter()
        .zip(plans)
        .map(|(rule, plan)| {
            let outputs = plan
                .observations
                .iter()
                .filter(|output| {
                    output.scope == ryo_values::domain::plan_parts::types::StepScope::PerFile
                        && plan.frontier.contains(&output.source)
                })
                .cloned()
                .collect::<Vec<_>>();
            let outputs = if outputs.is_empty() {
                ryo_values::domain::observations::types::ObservationOutputLayout::NoPersistentRoots
            } else {
                ryo_values::domain::observations::types::ObservationOutputLayout::Persistent {
                    configuration: rule.digest,
                    outputs,
                }
            };
            Ok(
                ryo_values::domain::observations::types::ObservationRuleLayout {
                    rule: rule.rule.clone(),
                    outputs,
                    frontiers: ryo_values::domain::observations::types::ObservationFrontierCount(
                        u32::try_from(plan.frontier.len())
                            .map_err(|_| RuleIntentError::TooManyOutputs)?,
                    ),
                },
            )
        })
        .collect::<Result<Vec<_>, RuleIntentError>>()?;
    let layout = digest.digest(DigestBytes(
        &ryo_values::domain::observations::service::layout_identity(&layout).0,
    ));
    for rule in &mut rules {
        rule.digest = digest.combine(&[rule.digest, layout]);
    }
    for consumer in order {
        let inputs = bindings
            .get(consumer.0)
            .ok_or(RuleIntentError::InvalidReference)?;
        if inputs.is_empty() {
            continue;
        }
        let own = rules
            .get(consumer.0)
            .ok_or(RuleIntentError::InvalidReference)?
            .clone();
        let mut parts = vec![
            own.digest,
            digest.digest(DigestBytes(OBSERVATION_PRODUCERS_DOMAIN)),
        ];
        let mut across = own.crossing == Crossing::Across;
        for input in inputs {
            across |=
                input.schema.scope == ryo_values::domain::plan_parts::types::StepScope::CrossFile;
            for producer in &input.producers {
                match producer {
                    ResolvedObservationProducer::Output { rule, .. } => {
                        let upstream =
                            rules.get(rule.0).ok_or(RuleIntentError::InvalidReference)?;
                        parts.push(upstream.digest);
                        across |= upstream.crossing == Crossing::Across;
                    }
                    // Explicit NoQuery has no executable digest; its selected identity
                    // is already included by the typed declaration codec.
                    ResolvedObservationProducer::NoQuery { .. } => {}
                }
            }
        }
        let result = rules
            .get_mut(consumer.0)
            .ok_or(RuleIntentError::InvalidReference)?;
        result.digest = digest.combine(&parts);
        if across {
            result.crossing = Crossing::Across;
        }
    }
    Ok(super::super::types::PreparedRuleIdentities {
        rules,
        observations:
            ryo_execution_domain::domain::prepared_request::types::ObservationIdentity::Named(
                ryo_values::domain::records::types::ObservationReadinessLayout(layout),
            ),
    })
}
