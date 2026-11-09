//! Rule compilation keeps the shared session cache bounded and evicts oldest-first.
use super::constants::COMPILED_PLANS;
use super::errors::RuleCompilationError;
use super::service::CompiledPlans;
use super::service::RuleCompilation;
use super::service_test::{population, rule};
use crate::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::runs::types::RuleId;
use std::sync::{Arc, PoisonError, atomic::Ordering};

/// One operation over a real compilation service and substituted compiler port.
type Input = fn() -> Result<Value, Failure>;
/// Cache population, compiler calls, and a scenario-specific invariant.
type Value = [usize; 3];
/// The exact compilation or request-binding refusal.
type Failure = RuleCompilationError;

/// Independently meaningful cache bound and eviction contracts.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("changed_ids_across_requests_keep_a_nonempty_bounded_session"),
        word: Word::Succeeds,
        input: || changed_ids,
        expect: Expect::Value([COMPILED_PLANS, (COMPILED_PLANS / 1024 + 2) * 1024, 1]),
    },
    Case {
        name: CaseName("oversized_requests_keep_exactly_the_cap_and_return_every_plan"),
        word: Word::Succeeds,
        input: || oversized_requests,
        expect: Expect::Value([COMPILED_PLANS, COMPILED_PLANS * 3 + 1, 1]),
    },
    Case {
        name: CaseName("separate_requests_evict_only_one_oldest_plan_at_a_time"),
        word: Word::Succeeds,
        input: || separate_request_eviction,
        expect: Expect::Value([COMPILED_PLANS, COMPILED_PLANS + 1, 1]),
    },
    Case {
        name: CaseName("compiled_plans_obeys_COMPILED_PLANS_and_preserves_FIFO_answers"),
        word: Word::Succeeds,
        input: || bounded,
        expect: Expect::Value([COMPILED_PLANS, COMPILED_PLANS + 1, 1]),
    },
];

/// Hold the returned invocation plans while FIFO eviction drops only the oldest shared entry.
fn bounded() -> Result<Value, Failure> {
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
fn changed_ids() -> Result<Value, Failure> {
    const PER_RUN: usize = 1024;
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let mut most = 0;
    for run in 0..COMPILED_PLANS / PER_RUN + 2 {
        let rules: Vec<_> = (0..PER_RUN)
            .map(|place| rule(&format!("{run}-{place}"), "one"))
            .collect();
        let answer = service.compile(&rules, &[])?;
        assert_eq!(answer.plans.len(), PER_RUN);
        most = most.max(population(&plans));
        assert!(population(&plans) <= COMPILED_PLANS);
    }
    Ok([
        population(&plans),
        compiler.calls.load(Ordering::SeqCst),
        usize::from(most > 0),
    ])
}

/// An invocation retains every answer even when its own ids overflow the shared cache.
fn oversized_requests() -> Result<Value, Failure> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    for (tag, count) in [("big", COMPILED_PLANS + 1), ("bigger", COMPILED_PLANS * 2)] {
        let rules: Vec<_> = (0..count)
            .map(|place| rule(&format!("{tag}-{place}"), "one"))
            .collect();
        let answer = service.compile(&rules, &[])?;
        assert_eq!(answer.plans.len(), count);
        assert_eq!(population(&plans), COMPILED_PLANS);
    }
    Ok([population(&plans), compiler.calls.load(Ordering::SeqCst), 1])
}

/// FIFO eviction after later requests preserves every entry except the oldest.
fn separate_request_eviction() -> Result<Value, Failure> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    for (tag, count) in [
        ("old", COMPILED_PLANS / 2),
        ("newer", COMPILED_PLANS - COMPILED_PLANS / 2),
        ("latest", 1),
    ] {
        let rules: Vec<_> = (0..count)
            .map(|place| rule(&format!("{tag}-{place}"), "one"))
            .collect();
        let answer = service.compile(&rules, &[])?;
        assert_eq!(answer.plans.len(), count);
    }
    let kept = plans.0.lock().unwrap_or_else(PoisonError::into_inner);
    let named = |tag: &str, place| {
        kept.plans
            .contains_key(&RuleId(format!("{tag}-{place}").into()))
    };
    let only_the_oldest_went = kept.plans.len() == COMPILED_PLANS
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

/// Run the service and preserve its typed result.
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
