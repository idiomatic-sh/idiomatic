//! The `rules` service's cases, on values alone.

use super::service::id_at;
use crate::domain::coverage::types::RuleIndex;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::runs::types::RuleId;

/// What a row of the cases does with the rules of a run.
#[derive(Clone, Copy)]
enum Scenario {
    IdAtPlace,
}

/// What a row of the cases does with the rules of a run.
type Input = Scenario;
/// The answer as up to three numbers, the first byte of an id standing for the id; zero where there is nothing.
type Value = [i64; 3];
type Failure = std::convert::Infallible;

fn byte(id: &str) -> i64 {
    i64::from(id.bytes().next().unwrap_or(0))
}

fn ids() -> Vec<RuleId> {
    ["a", "b", "c"]
        .iter()
        .map(|id| RuleId((*id).into()))
        .collect()
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::IdAtPlace => [byte(&id_at(&ids(), RuleIndex(2)).0), 0, 0],
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_id_at_a_place_is_the_id_of_the_rule_there"),
    word: Word::Succeeds,
    input: || Scenario::IdAtPlace,
    expect: Expect::Value([99, 0, 0]),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
