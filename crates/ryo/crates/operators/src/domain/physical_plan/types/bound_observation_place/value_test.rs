//! A bound observation place names its rule and its binding: two rules' equal bindings are two places.
use super::value::BoundObservationPlace;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_values::domain::plan_parts::types::ObservationBindingId;

type Input = (RuleOrdinal, RuleOrdinal);
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_rule_s_binding_is_one_place"),
        word: Word::Succeeds,
        input: || (RuleOrdinal(2), RuleOrdinal(2)),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("two_rules_bindings_are_two_places"),
        word: Word::Succeeds,
        input: || (RuleOrdinal(2), RuleOrdinal(3)),
        expect: Expect::Value(false),
    },
];

fn subject((left, right): Input) -> Result<Value, Failure> {
    Ok(BoundObservationPlace::of(left, ObservationBindingId(0))
        == BoundObservationPlace::of(right, ObservationBindingId(0)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
