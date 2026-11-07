//! Canonical authored declarations shared by invocation identity and durable case receipts.
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;

/// Executable dependency closure. NoQuery is knowledge, not an executable definition.
pub(crate) fn closure<'a>(
    seeds: impl IntoIterator<Item = ryo::linter::RuleId>,
    lookup: impl Fn(&ryo::linter::RuleId) -> Option<&'a [ryo::linter::ObservationBinding]>,
) -> std::collections::BTreeSet<ryo::linter::RuleId> {
    let mut found = std::collections::BTreeSet::new();
    let mut pending: Vec<_> = seeds.into_iter().collect();
    while let Some(id) = pending.pop() {
        if !found.insert(id.clone()) {
            continue;
        }
        for binding in lookup(&id).into_iter().flatten() {
            for producer in &binding.producers {
                if let ryo::linter::ObservationProducer::Output { rule, .. } = producer {
                    pending.push(rule.clone());
                }
            }
        }
    }
    found
}

/// Resolve only profile defaults; an explicit case declaration bypasses this policy.
pub(crate) fn project_profile(
    declarations: &RuleObservationDeclarations,
    selected: &[ryo::linter::RuleId],
    policy: ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy,
) -> RuleObservationDeclarations {
    if !selected.iter().any(|id| declarations.contains_key(id)) {
        return RuleObservationDeclarations::default();
    }
    let selected: std::collections::BTreeSet<_> = selected.iter().collect();
    let mut pending: Vec<ryo::linter::RuleId> = selected
        .iter()
        .filter(|id| declarations.contains_key(**id))
        .map(|id| (**id).clone())
        .collect();
    let mut resolved = RuleObservationDeclarations::default();
    while let Some(id) = pending.pop() {
        if resolved.contains_key(&id) {
            continue;
        }
        let Some(authored) = declarations.get(&id) else {
            continue;
        };
        let mut bindings = authored.clone();
        if policy
            == ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Selected
        {
            for binding in &mut bindings {
                binding.producers.retain(|producer| match producer {
                    ryo::linter::ObservationProducer::Output { rule, .. }
                    | ryo::linter::ObservationProducer::NoQuery { rule } => selected.contains(rule),
                });
            }
        }
        for binding in &bindings {
            for producer in &binding.producers {
                if let ryo::linter::ObservationProducer::Output { rule, .. } = producer {
                    if declarations.contains_key(rule) {
                        pending.push(rule.clone());
                    }
                }
            }
        }
        resolved.insert(id, bindings);
    }
    resolved
}
