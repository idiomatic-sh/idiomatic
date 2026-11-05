//! Rule compilation keeps bounded session plans and request-local bindings over injected ports.
use super::constants::COMPILED_PLANS;
use super::errors::RuleCompilationError;
use super::service::RuleCompilation;
use super::types::CompiledPlans;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::rules::errors::RulesError;
use crate::domain::rules::types::{};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::fixtures::fake::{QueryLanguageFake, REFUSED};
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_values::domain::descriptions::types::ValidateReport;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostedRelation,
};
use ryo_values::domain::plan_parts::types::{ParameterName, QueryText, WholeNumber};
use ryo_values::domain::plans::types::{ParameterType, Plan, PlanParameter};
use ryo_values::domain::program::types::QueryBundle;
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
        expect: Expect::Error(RuleCompilationError::Rules(RulesError::BlankRuleId)),
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
        RuleCompilationError::Rules(RulesError::MissingParameter { .. })
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
    Ok([population(&plans), compiler.calls.load(Ordering::SeqCst), 1])
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
        observations: &[ryo_values::domain::observations::types::BoundObservationSchema],
    ) -> Result<ValidateReport, QueryLanguageError> {
        self.compiler.validate(bundle, hosted, observations)
    }
    fn compile(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationSchema],
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

use ryo_values::domain::rule_requests::types::{RuleSpec,RuleParameters,Given};
