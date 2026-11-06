//! Rule compilation keeps bounded session plans and request-local bindings over injected ports.
use super::constants::COMPILED_PLANS;
use super::errors::RuleCompilationError;
use super::service::RuleCompilation;
use super::types::CompiledPlans;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::fixtures::fake::{QueryLanguageFake, REFUSED};
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::descriptions::types::ValidateReport;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostedRelation,
};
use ryo_values::domain::plan_parts::types::{ParameterName, QueryText, WholeNumber};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::program::types::QueryBundle;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_bindings::types::ParameterType;
use ryo_values::domain::query_bindings::types::PlanParameter;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use std::sync::{Arc, PoisonError, atomic::Ordering};

/// One operation over a real compilation service and substituted compiler port.
type Input = fn() -> Result<Value, Failure>;
/// Cache population, compiler calls, and a scenario-specific invariant.
type Value = [usize; 3];
/// The exact compilation or request-binding refusal.
type Failure = RuleCompilationError;

/// Independently meaningful cache, ownership, failure and binding contracts.
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
    Case {
        name: CaseName("one_cached_plan_binds_each_request_even_after_a_missing_binding"),
        word: Word::Succeeds,
        input: || request_bindings,
        expect: Expect::Value([1, 1, 1]),
    },
    Case {
        name: CaseName("query_and_hosted_schema_changes_replace_a_cached_plan"),
        word: Word::Succeeds,
        input: || program_identity,
        expect: Expect::Value([1, 3, 1]),
    },
    Case {
        name: CaseName(
            "immutable_module_catalog_snapshots_share_bytes_and_changed_contents_or_ids_miss_cache",
        ),
        word: Word::Succeeds,
        input: || module_catalog_identity,
        expect: Expect::Value([1, 4, 1]),
    },
    Case {
        name: CaseName("failed_queries_are_not_cached_and_keep_their_owner"),
        word: Word::Succeeds,
        input: || refused_program,
        expect: Expect::Value([1, 3, 1]),
    },
    Case {
        name: CaseName("compiler_can_access_shared_state_without_a_held_cache_lock"),
        word: Word::Succeeds,
        input: || unlocked_port,
        expect: Expect::Value([1, 1, 1]),
    },
    Case {
        name: CaseName("blank_rule_is_refused_before_any_compile"),
        word: Word::Refuses,
        input: || blank_id,
        expect: Expect::Error(RuleCompilationError::Rules(RuleRequestsError::BlankRuleId)),
    },
];

/// A legitimate parameter-free rule with exact query identity.
fn rule(id: &str, text: &str) -> RuleSpec {
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
fn population(plans: &CompiledPlans) -> usize {
    plans
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .plans
        .len()
}

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

/// A successful compile can survive a refused binding, but every later request binds its own value.
fn request_bindings() -> Result<Value, Failure> {
    let mut fake = QueryLanguageFake::default();
    fake.plan.parameters.push(PlanParameter {
        name: ParameterName("limit".into()),
        kind: ParameterType::Number,
    });
    let compiler = Arc::new(fake);
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let mut asked = rule("bound", "select");
    let refused = service
        .compile(&[asked.clone()], &[])
        .expect_err("a missing binding is refused");
    assert!(matches!(
        refused,
        RuleCompilationError::Rules(RuleRequestsError::MissingParameter { .. })
    ));
    asked.given.push(Given {
        name: ParameterName("limit".into()),
        value: ParameterBinding::Number(WholeNumber(3)),
    });
    let first = service.compile(&[asked.clone()], &[])?;
    asked.given.first_mut().expect("given parameter").value =
        ParameterBinding::Number(WholeNumber(9));
    let second = service.compile(&[asked], &[])?;
    Ok([
        population(&plans),
        compiler.calls.load(Ordering::SeqCst),
        usize::from(first.bindings != second.bindings),
    ])
}

/// The shared identity includes complete query bundles and the declared hosted schema.
fn program_identity() -> Result<Value, Failure> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let _first = service.compile(&[rule("same", "select")], &[])?;
    let _cached = service.compile(&[rule("same", "select")], &[])?;
    let _changed_query = service.compile(&[rule("same", "changed")], &[])?;
    let hosted = [HostRelation {
        name: HostName("external".into()),
        columns: vec![HostColumn {
            name: HostColumnName("value".into()),
            kind: HostKind::Number,
        }],
        rows: Vec::new(),
    }];
    let _changed_schema = service.compile(&[rule("same", "changed")], &hosted)?;
    let _cached_schema = service.compile(&[rule("same", "changed")], &hosted)?;
    let kept = plans.0.lock().unwrap_or_else(PoisonError::into_inner);
    let cached = &kept.plans[&RuleId("same".into())];
    assert_eq!(cached.query, rule("same", "changed").query);
    Ok([kept.plans.len(), compiler.calls.load(Ordering::SeqCst), 1])
}

/// The real bounded cache compares exact catalog contents, never allocation identity.
fn module_catalog_identity() -> Result<Value, Failure> {
    use ryo_values::domain::program::service::{document_source, query_bundle, source_id};
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let module = source_id(ryo_values::domain::program::types::QuerySpelling(
        "shared/module",
    ))
    .expect("nonblank module id");
    let mut asked = rule("catalog", "select");
    asked.query = query_bundle(
        asked.query.root,
        [document_source(module.clone(), "original bytes\nλ")],
    )
    .expect("unique module identity");
    let snapshot = asked.query.clone();
    assert!(Arc::ptr_eq(&snapshot.modules, &asked.query.modules));
    assert!(Arc::ptr_eq(&snapshot.root.text.0, &asked.query.root.text.0));
    let _first = service.compile(&[asked.clone()], &[])?;
    let mut equal_contents = asked.clone();
    equal_contents.query.modules = Arc::new((*asked.query.modules).clone());
    assert!(!Arc::ptr_eq(
        &equal_contents.query.modules,
        &asked.query.modules
    ));
    let _equal_snapshot_hit = service.compile(&[equal_contents], &[])?;
    assert_eq!(compiler.calls.load(Ordering::SeqCst), 1);
    Arc::make_mut(&mut asked.query.modules)
        .get_mut(&module)
        .unwrap()
        .text = "changed bytes".into();
    assert_eq!(&*snapshot.modules[&module].text, "original bytes\nλ");
    let _changed_bytes_miss = service.compile(&[asked.clone()], &[])?;
    assert_eq!(compiler.calls.load(Ordering::SeqCst), 2);
    let renamed_id = source_id(ryo_values::domain::program::types::QuerySpelling(
        "shared/renamed",
    ))
    .expect("nonblank renamed id");
    Arc::make_mut(&mut asked.query.modules)
        .get_mut(&module)
        .unwrap()
        .id = renamed_id.clone();
    let _changed_document_id_miss = service.compile(&[asked.clone()], &[])?;
    assert_eq!(compiler.calls.load(Ordering::SeqCst), 3);
    let renamed = Arc::make_mut(&mut asked.query.modules)
        .remove(&module)
        .unwrap();
    let _absent = Arc::make_mut(&mut asked.query.modules).insert(renamed_id, renamed);
    let _changed_id_miss = service.compile(&[asked.clone()], &[])?;
    let _stable_snapshot_hit = service.compile(&[asked.clone()], &[])?;
    let kept = plans.0.lock().unwrap_or_else(PoisonError::into_inner);
    let cached = &kept.plans[&asked.id].query;
    assert!(Arc::ptr_eq(&cached.modules, &asked.query.modules));
    assert!(Arc::ptr_eq(&cached.root.text.0, &asked.query.root.text.0));
    Ok([kept.plans.len(), compiler.calls.load(Ordering::SeqCst), 1])
}

/// Repeated refused compiles never become a cache hit, and a later valid query still succeeds.
fn refused_program() -> Result<Value, Failure> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    for _ in 0_u8..2_u8 {
        let error = service
            .compile(&[rule("owner", REFUSED)], &[])
            .expect_err("compiler refusal is preserved");
        assert!(matches!(error, RuleCompilationError::Query { id, .. } if id == "owner"));
        assert_eq!(population(&plans), 0);
    }
    let _answer = service.compile(&[rule("owner", "select")], &[])?;
    Ok([population(&plans), compiler.calls.load(Ordering::SeqCst), 1])
}

/// A substituted compiler that proves no service cache lock crosses the port call.
struct LockCheckingCompiler {
    /// The same session state the service owns.
    plans: CompiledPlans,
    /// Existing compiler fake for typed answers and observations.
    compiler: QueryLanguageFake,
}
impl QueryLanguagePort for LockCheckingCompiler {
    fn validate(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ValidateReport, QueryLanguageError> {
        self.compiler.validate(bundle, hosted, observations)
    }
    fn compile(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError> {
        let available = self
            .plans
            .0
            .try_lock()
            .expect("no cache lock may cross a compiler call");
        drop(available);
        self.compiler.compile(bundle, hosted, observations)
    }
}

/// Execute through the lock-checking port rather than inspecting implementation syntax.
fn unlocked_port() -> Result<Value, Failure> {
    let plans = CompiledPlans::default();
    let compiler = Arc::new(LockCheckingCompiler {
        plans: plans.clone(),
        compiler: QueryLanguageFake::default(),
    });
    let service = RuleCompilation::new(Arc::<LockCheckingCompiler>::clone(&compiler))
        .compiling_into(plans.clone());
    let _answer = service.compile(&[rule("lock", "select")], &[])?;
    Ok([
        population(&plans),
        compiler.compiler.calls.load(Ordering::SeqCst),
        1,
    ])
}

/// Identity validation is part of compilation and precedes any injected compiler invocation.
fn blank_id() -> Result<Value, Failure> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let result = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compile(&[rule("", "select")], &[]);
    assert_eq!(compiler.calls.load(Ordering::SeqCst), 0);
    result.map(|answer| [answer.plans.len(), 0, 0])
}

/// Run the service and preserve its typed result.
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rule_requests::types::{Given, RuleParameters, RuleSpec};
