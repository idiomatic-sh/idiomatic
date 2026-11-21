//! The one load of a run's selected rules, when the store holds no pack of what the run was given: every rule in one call of
//! the engine's `load`, each with what it binds its parameters to and its steps' digests, with the statistics the run was
//! given; the refused rules counted by why.

use super::super::types::{LoadRequest, Refusals};
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::DigestBytes;
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::lowering::types::{
    BoundProducer, LoadGiven, LoadRefusal, ObservedBinding, SelectedRule, SelectedRules,
};
use ryo_values::domain::observations::types::ObservationProducer;
use ryo_values::domain::plan_parts::types::{Input, ObservationBindingId};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::statistics::types::{MatchedStatistics, StatisticsBytes};
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_plans::service::{node_digests, reach_unfolded};
use ryo_values::domain::query_plans::types::{NodeDigest, NodeDigests, Plan};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// The digest of each step of a rule's plan under what the rule binds its parameters to, by the run's one digest: the
/// engine hashes nothing and takes these as data. The port's sixteen bytes are the node's digest.
pub(in crate::application::pack_load) fn step_digests(
    digest: &dyn DigestPort,
    plan: &Plan,
    bindings: RuleBindings<'_>,
) -> NodeDigests {
    node_digests(plan, bindings, &|bytes| {
        NodeDigest(digest.digest(DigestBytes(bytes.0)).0)
    })
}

/// The producers of each observation a selected rule's plan binds: the rules the binding names that the run selected, each
/// with the step of its plan the output it names is; and the digest the observation's definition is named by, made of the
/// binding's name and its producers' steps' digests. A producer the run did not select, or whose plan does not hand the
/// output over, hands over no rows.
fn observed(
    digest: &dyn DigestPort,
    request: &LoadRequest<'_>,
    rules: &[(&Plan, RuleBindings<'_>)],
    digests: &[NodeDigests],
) -> Vec<ObservedBinding> {
    let specs = request.declared.rules;
    let mut observed = Vec::new();
    for (consumer, ((plan, _), spec)) in rules.iter().zip(specs).enumerate() {
        for (place, input) in plan.observation_inputs.iter().enumerate() {
            let Some(binding) = spec.observations.iter().find(|each| each.name == input.name) else {
                continue;
            };
            let mut named: Vec<u8> = input.name.0.as_bytes().to_vec();
            let mut producers = Vec::new();
            for producer in &binding.producers {
                let ObservationProducer::Output { rule, output } = producer else {
                    continue;
                };
                let Some(at) = specs.iter().position(|each| each.id == *rule) else {
                    continue;
                };
                let source = rules.get(at).and_then(|(plan, _)| {
                    let observation = plan.observations.iter().find(|each| each.name == *output)?;
                    match observation.source {
                        Input::Step(step) => Some(step),
                        Input::Relation(_) | Input::BoundObservation(_) => None,
                    }
                });
                let Some(step) = source else {
                    continue;
                };
                let step_digest = digests
                    .get(at)
                    .and_then(|each| each.0.get(usize::from(step)).copied())
                    .unwrap_or_default();
                named.extend_from_slice(&step_digest.0);
                producers.push(BoundProducer {
                    rule: SelectedRuleOrdinal(u32::try_from(at).unwrap_or(u32::MAX)),
                    step,
                });
            }
            observed.push(ObservedBinding {
                consumer: SelectedRuleOrdinal(u32::try_from(consumer).unwrap_or(u32::MAX)),
                binding: ObservationBindingId(u32::try_from(place).unwrap_or(u32::MAX)),
                node: NodeDigest(digest.digest(DigestBytes(&named)).0),
                producers,
            });
        }
    }
    observed
}

/// What one load made of a request's rules.
pub(in crate::application::pack_load) struct Lowered {
    /// The programs of the accepted rules.
    pub(in crate::application::pack_load) programs: Programs,
    /// The refused rules, by why.
    pub(in crate::application::pack_load) refusals: Refusals,
    /// What the statistics name of the programs.
    pub(in crate::application::pack_load) matched: MatchedStatistics,
}

/// Every selected rule of `request` lowered in one load: a sub-plan two rules share is lowered once, and each refused rule
/// is answered with why. The statistics reach the engine as the request gives them: no bytes are no statistics.
pub(in crate::application::pack_load) fn lowered(
    engine: &dyn EnginePort,
    digest: &dyn DigestPort,
    request: &LoadRequest<'_>,
) -> Lowered {
    // Each rule with what it binds its parameters to (nothing past the request's bindings) and its steps' digests.
    let unbound = std::iter::repeat(&[][..]);
    let bound = request.bindings.iter().map(Vec::as_slice).chain(unbound);
    // The new engine receives each plan with its joins' reaches unfolded into steps of the plan, so the digests are of the
    // steps it lowers. Today's engine keeps the request's plans.
    let unfolded: Vec<Cow<'_, Plan>> = request
        .plans
        .iter()
        .map(|plan| reach_unfolded(plan))
        .collect();
    let rules: Vec<(&Plan, RuleBindings<'_>)> = unfolded
        .iter()
        .zip(bound)
        .map(|(plan, bound)| (&**plan, RuleBindings(bound)))
        .collect();
    let digests: Vec<NodeDigests> = rules
        .iter()
        .map(|(plan, bound)| step_digests(digest, plan, *bound))
        .collect();
    let selected: Vec<SelectedRule<'_>> = rules
        .iter()
        .zip(&digests)
        .enumerate()
        .map(|(place, ((plan, bound), digests))| SelectedRule {
            rule: SelectedRuleOrdinal(u32::try_from(place).unwrap_or(u32::MAX)),
            plan,
            bindings: *bound,
            digests,
        })
        .collect();
    let statistics = StatisticsBytes(request.statistics.0);
    let observed = observed(digest, request, &rules, &digests);
    let given = LoadGiven {
        hosted: request.declared.hosted,
        observed: &observed,
    };
    let source = LoadSource::Plans(SelectedRules(&selected), statistics, given);
    match engine.load(source, request.kinds) {
        Ok(loaded) => {
            // Spike (ryo-fmx3.1): each refused rule with why, when the run asks for them.
            if std::env::var_os("RYO_L2_REFUSED").is_some() {
                for refused in &loaded.refused {
                    eprintln!("ryo l2: refused rule {}: {}", refused.rule.0, refused.refusal);
                }
            }
            let mut refusals = Refusals::default();
            for refused in &loaded.refused {
                let count = match refused.refusal {
                    LoadRefusal::Unlowerable(_) => &mut refusals.unlowerable,
                    LoadRefusal::Budget(_) => &mut refusals.past_width,
                };
                *count = Count(count.0.saturating_add(1));
            }
            Lowered {
                programs: loaded.programs,
                refusals,
                matched: loaded.matched,
            }
        }
        // Plans are never a pack's fault, and refused fixed names leave no words: no rule loads, each counted as not lowered.
        Err(
            LoadError::PackVersion
            | LoadError::PackDigest
            | LoadError::PackCorrupt
            | LoadError::Vocabulary(_),
        ) => Lowered {
            programs: Programs::default(),
            refusals: Refusals {
                unlowerable: Count(selected.len()),
                past_width: Count(0),
            },
            matched: MatchedStatistics::Absent,
        },
    }
}
