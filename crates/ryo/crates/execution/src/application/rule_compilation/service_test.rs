//! Rule compilation keeps bounded session plans and request-local bindings over injected ports.
use super::constants::COMPILED_PLANS;
use super::errors::RuleCompilationError;
use super::service::CompiledPlans;
use super::service::RuleCompilation;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::fixtures::fake::{QueryLanguageFake, REFUSED};
use crate::outbound::query_language::interface::QueryLanguagePort;
use fixtures::{
    Compiled, bounded, changed_ids, module_catalog_identity, oversized_requests, population,
    refused_program, rule, separate_request_eviction, unlocked_port,
};
use ryo_execution_domain::domain::operator::constants::{OPERATORS, describe};
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::descriptions::types::ValidateReport;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostedRelation,
};
use ryo_values::domain::plan_parts::types::{ParameterName, QueryText, WholeNumber};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_bindings::types::ParameterType;
use ryo_values::domain::query_bindings::types::PlanParameter;
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use ryo_values::domain::runs::types::RuleId;
use std::borrow::Cow;
use std::sync::{Arc, PoisonError, atomic::Ordering};

/// One operation over a real compilation service and substituted compiler port.
type Input = fn() -> Result<Value, Failure>;
/// Cache population, compiler calls, and the scenario's further observations.
type Value = Compiled;
/// The exact compilation or request-binding refusal.
type Failure = RuleCompilationError;

/// Independently meaningful cache, ownership, failure and binding contracts.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_cached_plan_binds_each_request_even_after_a_missing_binding"),
        word: Word::Succeeds,
        input: || request_bindings,
        expect: Expect::Value(Compiled {
            kept: 1,
            calls: 1,
            observed: Cow::Borrowed(&[
                ("refused for a missing binding", 1),
                ("requests bound apart", 1),
            ]),
        }),
    },
    Case {
        name: CaseName("query_and_hosted_schema_changes_replace_a_cached_plan"),
        word: Word::Succeeds,
        input: || program_identity,
        expect: Expect::Value(Compiled {
            kept: 1,
            calls: 3,
            observed: Cow::Borrowed(&[("the changed query is cached", 1)]),
        }),
    },
    Case {
        name: CaseName(
            "immutable_module_catalog_snapshots_share_bytes_and_changed_contents_or_ids_miss_cache",
        ),
        word: Word::Succeeds,
        input: || module_catalog_identity,
        expect: Expect::Value(Compiled {
            kept: 1,
            calls: 4,
            observed: Cow::Borrowed(&[
                ("the snapshot shares the modules", 1),
                ("the snapshot shares the root text", 1),
                ("equal contents share the modules", 0),
                ("calls after equal contents", 1),
                ("the snapshot keeps the original bytes", 1),
                ("calls after changed bytes", 2),
                ("calls after a changed document id", 3),
                ("the cache shares the modules", 1),
                ("the cache shares the root text", 1),
            ]),
        }),
    },
    Case {
        name: CaseName("failed_queries_are_not_cached_and_keep_their_owner"),
        word: Word::Succeeds,
        input: || refused_program,
        expect: Expect::Value(Compiled {
            kept: 1,
            calls: 3,
            observed: Cow::Borrowed(&[
                ("refused for its owner", 1),
                ("kept after the refusal", 0),
                ("refused for its owner", 1),
                ("kept after the refusal", 0),
            ]),
        }),
    },
    Case {
        name: CaseName("compiler_can_access_shared_state_without_a_held_cache_lock"),
        word: Word::Succeeds,
        input: || unlocked_port,
        expect: Expect::Value(Compiled {
            kept: 1,
            calls: 1,
            observed: Cow::Borrowed(&[("calls under a held cache lock", 0)]),
        }),
    },
    Case {
        name: CaseName("a_blank_rule_never_reaches_the_compiler"),
        word: Word::Succeeds,
        input: || blank_id_calls,
        expect: Expect::Value(Compiled {
            kept: 0,
            calls: 0,
            observed: Cow::Borrowed(&[("refused", 1)]),
        }),
    },
    Case {
        name: CaseName("blank_rule_is_refused_before_any_compile"),
        word: Word::Refuses,
        input: || blank_id,
        expect: Expect::Error(RuleCompilationError::Rules(RuleRequestsError::BlankRuleId)),
    },
    Case {
        name: CaseName("changed_ids_across_requests_keep_a_nonempty_bounded_session"),
        word: Word::Succeeds,
        input: || changed_ids,
        expect: Expect::Value(Compiled {
            kept: COMPILED_PLANS,
            calls: (COMPILED_PLANS / 1024 + 2) * 1024,
            observed: Cow::Borrowed(&[
                ("most kept", COMPILED_PLANS),
                ("requests answered short", 0),
                ("requests leaving more than the cap", 0),
            ]),
        }),
    },
    Case {
        name: CaseName("oversized_requests_keep_exactly_the_cap_and_return_every_plan"),
        word: Word::Succeeds,
        input: || oversized_requests,
        expect: Expect::Value(Compiled {
            kept: COMPILED_PLANS,
            calls: COMPILED_PLANS * 3 + 1,
            observed: Cow::Borrowed(&[
                ("big", COMPILED_PLANS + 1),
                ("kept after it", COMPILED_PLANS),
                ("bigger", COMPILED_PLANS * 2),
                ("kept after it", COMPILED_PLANS),
            ]),
        }),
    },
    Case {
        name: CaseName("separate_requests_evict_only_one_oldest_plan_at_a_time"),
        word: Word::Succeeds,
        input: || separate_request_eviction,
        expect: Expect::Value(Compiled {
            kept: COMPILED_PLANS,
            calls: COMPILED_PLANS + 1,
            observed: Cow::Borrowed(&[
                ("old", COMPILED_PLANS / 2),
                ("newer", COMPILED_PLANS - COMPILED_PLANS / 2),
                ("latest", 1),
                ("old-0 kept", 0),
                ("old-1 kept", 1),
                ("newer-0 kept", 1),
                ("last newer kept", 1),
                ("latest-0 kept", 1),
            ]),
        }),
    },
    Case {
        name: CaseName("compiled_plans_obeys_COMPILED_PLANS_and_preserves_FIFO_answers"),
        word: Word::Succeeds,
        input: || bounded,
        expect: Expect::Value(Compiled {
            kept: COMPILED_PLANS,
            calls: COMPILED_PLANS + 1,
            observed: Cow::Borrowed(&[
                ("answered", COMPILED_PLANS + 1),
                ("rule-0 kept", 0),
                ("rule-1 kept", 1),
            ]),
        }),
    },
];

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
    let refused = service.compile(&[asked.clone()], &[]).err();
    let missing = usize::from(matches!(
        refused,
        Some(RuleCompilationError::Rules(
            RuleRequestsError::MissingParameter { .. }
        ))
    ));
    asked.given.push(Given {
        name: ParameterName("limit".into()),
        value: ParameterBinding::Number(WholeNumber(3)),
    });
    let first = service.compile(&[asked.clone()], &[])?;
    asked.given.first_mut().expect("given parameter").value =
        ParameterBinding::Number(WholeNumber(9));
    let second = service.compile(&[asked], &[])?;
    Ok(Compiled {
        kept: population(&plans),
        calls: compiler.calls.load(Ordering::SeqCst),
        observed: vec![
            ("refused for a missing binding", missing),
            (
                "requests bound apart",
                usize::from(first.bindings != second.bindings),
            ),
        ]
        .into(),
    })
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
    Ok(Compiled {
        kept: kept.plans.len(),
        calls: compiler.calls.load(Ordering::SeqCst),
        observed: vec![(
            "the changed query is cached",
            usize::from(cached.query == rule("same", "changed").query),
        )]
        .into(),
    })
}

/// Identity validation is part of compilation and precedes any injected compiler invocation.
fn blank_id() -> Result<Value, Failure> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let result = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compile(&[rule("", "select")], &[]);
    result.map(|answer| Compiled {
        kept: answer.plans.len(),
        calls: compiler.calls.load(Ordering::SeqCst),
        observed: Cow::Borrowed(&[]),
    })
}

/// The compiler's calls after a blank rule's compile, and whether it was refused.
fn blank_id_calls() -> Result<Value, Failure> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let refused = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compile(&[rule("", "select")], &[])
        .err()
        .map_or(0, |_| 1);
    Ok(Compiled {
        kept: 0,
        calls: compiler.calls.load(Ordering::SeqCst),
        observed: vec![("refused", refused)].into(),
    })
}

/// Run the service and preserve its typed result.
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}

/// The rows of the examples the operators print: no input, and the count of examples that do not compile as the value.
const EXAMPLES: &[Case<(), usize, QueryLanguageError>] = &[Case {
    name: CaseName("every_operator_example_compiles_as_it_is_printed"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(0),
}];

/// How many of the examples the operators print do not compile through the real query language adapter.
fn uncompiled() -> usize {
    let compile = Arc::new(CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let typecheck = Arc::new(TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let adapter = crate::outbound::query_language::factory::use_case(compile, typecheck);
    OPERATORS
        .iter()
        .filter(|operator| {
            let described = describe(**operator);
            matches!(
                adapter.compile(
                    &QueryBundle::from(QueryText(described.example.0.clone())),
                    &[],
                    &[]
                ),
                Err(QueryLanguageError::Refused { .. })
            )
        })
        .count()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(EXAMPLES, Word::Succeeds, |()| Ok(uncompiled()));
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rule_requests::types::Given;
#[cfg(test)]
mod fixtures;
