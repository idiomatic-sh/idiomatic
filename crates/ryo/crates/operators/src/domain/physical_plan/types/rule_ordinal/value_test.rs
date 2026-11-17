//! A rule's next place is the place after it, and the last place has no place after it.
use super::value::RuleOrdinal;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = RuleOrdinal;
type Value = RuleOrdinal;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_next_rule_is_the_one_after"),
        word: Word::Succeeds,
        input: || RuleOrdinal(4),
        expect: Expect::Value(RuleOrdinal(5)),
    },
    Case {
        name: CaseName("the_last_place_stays_the_last"),
        word: Word::Succeeds,
        input: || RuleOrdinal(u32::MAX),
        expect: Expect::Value(RuleOrdinal(u32::MAX)),
    },
];

fn subject(rule: Input) -> Result<Value, Failure> {
    Ok(rule.next())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
