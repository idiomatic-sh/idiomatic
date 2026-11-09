//! The `rule_requests` service's cases, on values alone.

use super::errors::RuleRequestsError;
use super::service::{bind, check_ids, failed};
use super::types::{Given, RuleParameters, RuleSpec};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::files::types::Glob;
use crate::domain::plan_parts::types::{
    ColumnIndex, ParameterName, QueryText, StepId, WholeNumber,
};
use crate::domain::plans::types::{Emit, Plan};
use crate::domain::query_bindings::types::ParameterBinding;
use crate::domain::query_bindings::types::ParameterType;
use crate::domain::query_bindings::types::PlanParameter;
use crate::domain::runs::types::RuleId;

/// What a row of the cases does with the rules of a run.
#[derive(Clone, Copy)]
enum Scenario {
    CheckedIds,
    BlankId,
    DuplicateId,
    Bound,
    Unbound,
    Failed,
}

/// What a row of the cases does with the rules of a run.
type Input = Scenario;
/// The answer as up to three numbers; zero where there is nothing.
type Value = [i64; 3];
type Failure = RuleRequestsError;

fn globs(texts: &[&str]) -> Vec<Glob> {
    texts.iter().map(|text| Glob((*text).into())).collect()
}

fn spec(id: &str, include: &[&str], exclude: &[&str]) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: crate::domain::query_tokens::types::QueryBundle::from(QueryText("select".into())),
        applicability: crate::domain::runs::types::RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
        spelled: RuleParameters("limit=3".into()),
        include: globs(include),
        exclude: globs(exclude),
    }
}

fn parameter(name: &str) -> PlanParameter {
    PlanParameter {
        name: ParameterName(name.into()),
        kind: ParameterType::Number,
    }
}

fn given(name: &str, number: i64) -> Given {
    Given {
        name: ParameterName(name.into()),
        value: ParameterBinding::Number(WholeNumber(number)),
    }
}

fn plan_with(parameters: Vec<PlanParameter>) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters,
        steps: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let rules = [
        spec("a", &["*.rs"], &[]),
        spec("b", &[], &["t/**", "x/**"]),
        spec("c", &[], &[]),
    ];
    let id = RuleId("a".into());
    Ok(match scenario {
        Scenario::CheckedIds => {
            check_ids(&rules)?;
            [3, 0, 0]
        }
        Scenario::BlankId => {
            check_ids(&[spec("a", &[], &[]), spec("  ", &[], &[])])?;
            [0, 0, 0]
        }
        Scenario::DuplicateId => {
            check_ids(&[
                spec("a", &[], &[]),
                spec("b", &[], &[]),
                spec("a", &[], &[]),
            ])?;
            [0, 0, 0]
        }
        Scenario::Bound => {
            let mut rule = spec("a", &[], &[]);
            rule.given = vec![given("first", 1), given("second", 2)];
            let plan = plan_with(vec![parameter("second"), parameter("first")]);
            let mut bound = [0; 3];
            for (slot, binding) in bound.iter_mut().zip(bind(&plan, &rule)?) {
                *slot = match binding {
                    ParameterBinding::Number(number) => number.0,
                    ParameterBinding::Flag(_)
                    | ParameterBinding::Text(_)
                    | ParameterBinding::Numbers(_)
                    | ParameterBinding::Texts(_)
                    | ParameterBinding::TextSets(_)
                    | ParameterBinding::Tables(_) => -1,
                };
            }
            bound
        }
        Scenario::Unbound => {
            let mut rule = spec("a", &[], &[]);
            rule.given = vec![given("first", 1)];
            let plan = plan_with(vec![parameter("first"), parameter("limit")]);
            let bound = bind(&plan, &rule)?;
            [i64::try_from(bound.len()).unwrap_or(i64::MAX), 0, 0]
        }
        Scenario::Failed => return Err(failed(&id, &"boom")),
    })
}

fn matches_id_and_reason(error: &RuleRequestsError, want: &str) -> bool {
    match error {
        RuleRequestsError::Failed { id, reason } => &*id.0 == "a" && reason == want,
        RuleRequestsError::BlankRuleId
        | RuleRequestsError::DuplicateRuleId { .. }
        | RuleRequestsError::MissingParameter { .. } => false,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rules_with_different_ids_are_accepted"),
        word: Word::Succeeds,
        input: || Scenario::CheckedIds,
        expect: Expect::Value([3, 0, 0]),
    },
    Case {
        name: CaseName("a_rule_gets_its_parameters_by_the_place_the_plan_reads_them_at"),
        word: Word::Succeeds,
        input: || Scenario::Bound,
        expect: Expect::Value([2, 1, 0]),
    },
    Case {
        name: CaseName("blank_rule_id_is_refused"),
        word: Word::Refuses,
        input: || Scenario::BlankId,
        expect: Expect::Error(RuleRequestsError::BlankRuleId),
    },
    Case {
        name: CaseName("duplicate_rule_id_is_refused_naming_the_id"),
        word: Word::Refuses,
        input: || Scenario::DuplicateId,
        expect: Expect::Refusal(|error| {
            if matches!(error, RuleRequestsError::DuplicateRuleId { id } if &*id.0 == "a") {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_parameter_nothing_is_bound_to_is_refused_naming_the_rule"),
        word: Word::Refuses,
        input: || Scenario::Unbound,
        expect: Expect::Refusal(|error| {
            if matches!(error, RuleRequestsError::MissingParameter { id } if &*id.0 == "a") {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_rule_that_could_not_run_fails_with_the_words_of_what_refused_it"),
        word: Word::Fails,
        input: || Scenario::Failed,
        expect: Expect::Refusal(|error| {
            if matches!(error, RuleRequestsError::Failed { .. })
                && matches_id_and_reason(error, "boom")
            {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
