//! The `rules` service's cases, on values alone.

use super::service::{governing, ids_of, owner_of, scopes_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution::inbound::use_cases::metadata::interface::{
    GlobOwner, GlobSide, Mask, ScopePlacement,
};
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::RuleNumber;

/// What a row of the cases does with the rules of a run.
#[derive(Clone, Copy)]
enum Scenario {
    Ids,
    Scopes,
    OwnerOfOwned,
    OwnerOfNobody,
    Governing,
}

/// What a row of the cases does with the rules of a run.
type Input = Scenario;
/// The answer as up to three numbers, the first byte of an id standing for the id; zero where there is nothing.
type Value = [i64; 3];
type Failure = std::convert::Infallible;

fn byte(id: &str) -> i64 {
    i64::from(id.bytes().next().unwrap_or(0))
}

fn globs(texts: &[&str]) -> Vec<Glob> {
    texts.iter().map(|text| Glob((*text).into())).collect()
}

fn spec(id: &str, include: &[&str], exclude: &[&str]) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(
            "select".into(),
        )),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
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
        Scenario::Governing => {
            let mut read = [0; 3];
            for (slot, id) in
                read.iter_mut()
                    .zip(governing(&MetadataUseCaseImpl, &Mask(vec![0b101]), &ids()))
            {
                *slot = byte(&id.0);
            }
            read
        }
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
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
        name: CaseName("the_rules_whose_bit_is_set_govern_the_file"),
        word: Word::Succeeds,
        input: || Scenario::Governing,
        expect: Expect::Value([97, 99, 0]),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
