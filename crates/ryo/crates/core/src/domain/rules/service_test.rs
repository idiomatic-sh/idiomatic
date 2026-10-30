//! The `rules` service's cases, on values alone.

use super::errors::RulesError;
use super::service::{bind, check_ids, failed, governing, id_at, ids_of, owner_of, scopes_of};
use super::types::{Given, RuleParameters, RuleSpec};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::finish::types::RuleNumber;
use crate::domain::placement::types::{GlobOwner, GlobSide, Mask, ScopePlacement};
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ParameterName, QueryText, StepId, WholeNumber,
};
use ryo_values::domain::plans::types::{Emit, ParameterType, Plan, PlanParameter};
use ryo_values::domain::runs::types::RuleId;

/// What a row of the cases does with the rules of a run.
#[derive(Clone, Copy)]
enum Scenario {
    /// Checks the ids of three rules that differ.
    CheckedIds,
    /// Checks the ids of rules, one of which has an id of blanks.
    BlankId,
    /// Checks the ids of rules, the third of which has the id of the first.
    DuplicateId,
    /// Binds the two parameters of a plan, declared in the other order than the rule gives them.
    Bound,
    /// Binds a plan that declares a parameter the rule gives nothing for.
    Unbound,
    /// Reads the ids of three rules.
    Ids,
    /// Reads how many include and exclude globs each of three rules has.
    Scopes,
    /// Asks whose a glob of the placement is.
    OwnerOfOwned,
    /// Asks whose a glob is that nobody owns.
    OwnerOfNobody,
    /// Asks the id of the rule at a place the rules have.
    IdAtPlace,
    /// Asks which of three rules govern the file of a mask.
    Governing,
    /// Says why a rule failed.
    Failed,
}

/// What a row of the cases does with the rules of a run.
type Input = Scenario;
/// The answer as up to three numbers, the first byte of an id standing for the id; zero where there is nothing.
type Value = [i64; 3];
type Failure = RulesError;

fn byte(id: &str) -> i64 {
    i64::from(id.bytes().next().unwrap_or(0))
}

fn globs(texts: &[&str]) -> Vec<Glob> {
    texts.iter().map(|text| Glob((*text).into())).collect()
}

fn spec(id: &str, include: &[&str], exclude: &[&str]) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: QueryText("select".into()),
        given: Vec::new(),
        spelled: RuleParameters("limit=3".into()),
        include: globs(include),
        exclude: globs(exclude),
    }
}

fn ids() -> Vec<RuleId> {
    ["a", "b", "c"]
        .iter()
        .map(|id| RuleId((*id).into()))
        .collect()
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
        parameters,
        steps: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    }
}

fn placement() -> ScopePlacement {
    ScopePlacement {
        globs: globs(&["*.rs", "*.md"]),
        owners: vec![
            vec![GlobOwner {
                rule: RuleNumber(1),
                side: GlobSide::Include,
            }],
            vec![GlobOwner {
                rule: RuleNumber(0),
                side: GlobSide::Exclude,
            }],
        ],
        always: Mask::default(),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let rules = [
        spec("a", &["*.rs"], &[]),
        spec("b", &[], &["t/**", "x/**"]),
        spec("c", &[], &[]),
    ];
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
        Scenario::Ids => {
            let mut read = [0; 3];
            for (slot, id) in read.iter_mut().zip(ids_of(&rules)) {
                *slot = byte(&id.0);
            }
            read
        }
        Scenario::Scopes => {
            let mut read = [0; 3];
            for (slot, scope) in read.iter_mut().zip(scopes_of(&rules)) {
                *slot = i64::try_from(scope.include.len() * 10 + scope.exclude.len())
                    .unwrap_or(i64::MAX);
            }
            read
        }
        Scenario::OwnerOfOwned => {
            let (placed, named) = (placement(), ids());
            let owner = owner_of(&placed, &Glob("*.md".into()), &named);
            [owner.map_or(0, |id| byte(&id.0)), 0, 0]
        }
        Scenario::OwnerOfNobody => {
            let (placed, named) = (placement(), ids());
            let owner = owner_of(&placed, &Glob("*.py".into()), &named);
            [owner.map_or(0, |id| byte(&id.0)), 0, 0]
        }
        Scenario::IdAtPlace => [byte(&id_at(&ids(), RuleIndex(2)).0), 0, 0],
        Scenario::Governing => {
            let mut read = [0; 3];
            for (slot, id) in read.iter_mut().zip(governing(&Mask(vec![0b101]), &ids())) {
                *slot = byte(&id.0);
            }
            read
        }
        Scenario::Failed => return Err(failed(&RuleId("a".into()), &"boom")),
    })
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
        name: CaseName("the_ids_are_read_in_the_order_of_the_rules"),
        word: Word::Succeeds,
        input: || Scenario::Ids,
        expect: Expect::Value([97, 98, 99]),
    },
    Case {
        name: CaseName("the_globs_of_each_rule_are_its_scope"),
        word: Word::Succeeds,
        input: || Scenario::Scopes,
        expect: Expect::Value([10, 2, 0]),
    },
    Case {
        name: CaseName("the_rule_that_owns_a_glob_is_named"),
        word: Word::Succeeds,
        input: || Scenario::OwnerOfOwned,
        expect: Expect::Value([97, 0, 0]),
    },
    Case {
        name: CaseName("a_glob_nobody_owns_has_no_owner"),
        word: Word::Succeeds,
        input: || Scenario::OwnerOfNobody,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("the_id_at_a_place_is_the_id_of_the_rule_there"),
        word: Word::Succeeds,
        input: || Scenario::IdAtPlace,
        expect: Expect::Value([99, 0, 0]),
    },
    Case {
        name: CaseName("the_rules_whose_bit_is_set_govern_the_file"),
        word: Word::Succeeds,
        input: || Scenario::Governing,
        expect: Expect::Value([97, 99, 0]),
    },
    Case {
        name: CaseName("blank_rule_id_is_refused"),
        word: Word::Refuses,
        input: || Scenario::BlankId,
        expect: Expect::Error(RulesError::BlankRuleId),
    },
    Case {
        name: CaseName("duplicate_rule_id_is_refused_naming_the_id"),
        word: Word::Refuses,
        input: || Scenario::DuplicateId,
        expect: Expect::Refusal(|error| {
            if matches!(error, RulesError::DuplicateRuleId { id } if &*id.0 == "a") {
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
            if matches!(error, RulesError::MissingParameter { id } if &*id.0 == "a") {
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
            if matches!(
                error,
                RulesError::Failed { id, reason } if &*id.0 == "a" && reason == "boom"
            ) {
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
