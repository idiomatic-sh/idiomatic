//! The rule, cache population and eviction helpers the rule compilation cases share.
use super::super::constants::COMPILED_PLANS;
use super::super::errors::RuleCompilationError;
use super::super::service::CompiledPlans;
use super::super::service::RuleCompilation;
use crate::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use std::sync::PoisonError;
use std::sync::{Arc, atomic::Ordering};

/// A legitimate parameter-free rule with exact query identity.
pub(super) fn rule(id: &str, text: &str) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: QueryBundle::from(QueryText(text.into())),
        applicability: RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
        spelled: RuleParameters("".into()),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

/// The shared cache's actual population after an operation.
pub(super) fn population(plans: &CompiledPlans) -> usize {
    plans
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .plans
        .len()
}

/// Hold the returned invocation plans while FIFO eviction drops only the oldest shared entry.
pub(super) fn bounded() -> Result<[usize; 3], RuleCompilationError> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let rules: Vec<_> = (0..=COMPILED_PLANS)
        .map(|index| rule(&format!("rule-{index}"), "select"))
        .collect();
    let answer = service.compile(&rules, &[])?;
    let kept = plans.0.lock().unwrap_or_else(PoisonError::into_inner);
    let invariant = answer.plans.len() == COMPILED_PLANS + 1
        && !kept.plans.contains_key(&RuleId("rule-0".into()))
        && kept.plans.contains_key(&RuleId("rule-1".into()));
    Ok([
        kept.plans.len(),
        compiler.calls.load(Ordering::SeqCst),
        usize::from(invariant),
    ])
}

/// Distinct ids exercise the shared cache across several complete requests.
pub(super) fn changed_ids() -> Result<[usize; 3], RuleCompilationError> {
    const PER_RUN: usize = 1024;
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let mut most = 0;
    let mut answered_and_bounded = true;
    for run in 0..COMPILED_PLANS / PER_RUN + 2 {
        let rules: Vec<_> = (0..PER_RUN)
            .map(|place| rule(&format!("{run}-{place}"), "one"))
            .collect();
        let answer = service.compile(&rules, &[])?;
        most = most.max(population(&plans));
        answered_and_bounded &=
            answer.plans.len() == PER_RUN && population(&plans) <= COMPILED_PLANS;
    }
    Ok([
        population(&plans),
        compiler.calls.load(Ordering::SeqCst),
        usize::from(most > 0 && answered_and_bounded),
    ])
}

/// An invocation retains every answer even when its own ids overflow the shared cache.
pub(super) fn oversized_requests() -> Result<[usize; 3], RuleCompilationError> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let mut answered_and_full = true;
    for (tag, count) in [("big", COMPILED_PLANS + 1), ("bigger", COMPILED_PLANS * 2)] {
        let rules: Vec<_> = (0..count)
            .map(|place| rule(&format!("{tag}-{place}"), "one"))
            .collect();
        let answer = service.compile(&rules, &[])?;
        answered_and_full &= answer.plans.len() == count && population(&plans) == COMPILED_PLANS;
    }
    Ok([
        population(&plans),
        compiler.calls.load(Ordering::SeqCst),
        usize::from(answered_and_full),
    ])
}

/// FIFO eviction after later requests preserves every entry except the oldest.
pub(super) fn separate_request_eviction() -> Result<[usize; 3], RuleCompilationError> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let mut answered = true;
    for (tag, count) in [
        ("old", COMPILED_PLANS / 2),
        ("newer", COMPILED_PLANS - COMPILED_PLANS / 2),
        ("latest", 1),
    ] {
        let rules: Vec<_> = (0..count)
            .map(|place| rule(&format!("{tag}-{place}"), "one"))
            .collect();
        let answer = service.compile(&rules, &[])?;
        answered &= answer.plans.len() == count;
    }
    let kept = plans.0.lock().unwrap_or_else(PoisonError::into_inner);
    let named = |tag: &str, place| {
        kept.plans
            .contains_key(&RuleId(format!("{tag}-{place}").into()))
    };
    let only_the_oldest_went = answered
        && kept.plans.len() == COMPILED_PLANS
        && !named("old", 0)
        && named("old", 1)
        && named("newer", 0)
        && named("newer", COMPILED_PLANS - COMPILED_PLANS / 2 - 1)
        && named("latest", 0);
    Ok([
        kept.plans.len(),
        compiler.calls.load(Ordering::SeqCst),
        usize::from(only_the_oldest_went),
    ])
}
