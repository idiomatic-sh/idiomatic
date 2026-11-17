//! A frontier place names its rule and its entry: two rules' equal entries are two places.
use super::value::FrontierPlace;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_values::domain::plan_parts::types::InputNumber;

type Input = (RuleOrdinal, RuleOrdinal);
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_rule_s_entry_is_one_place"),
        word: Word::Succeeds,
        input: || (RuleOrdinal(0), RuleOrdinal(0)),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("two_rules_entries_are_two_places"),
        word: Word::Succeeds,
        input: || (RuleOrdinal(0), RuleOrdinal(1)),
        expect: Expect::Value(false),
    },
];

fn subject((left, right): Input) -> Result<Value, Failure> {
    Ok(FrontierPlace::of(left, InputNumber(0)) == FrontierPlace::of(right, InputNumber(0)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
