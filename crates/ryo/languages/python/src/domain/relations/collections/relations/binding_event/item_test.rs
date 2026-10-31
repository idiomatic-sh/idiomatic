//! Canonical typed-reader contract and explicit frontend refusal.
use super::super::fixtures::contract::check;
use super::item::BindingEventItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::proposed;
use ryo::languages::contract::Reading;
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty module has complete empty binding facts"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("malformed source does not prove absence"),
        word: Word::Succeeds,
        input: || "if:\n    pass\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["binding analysis requires complete supported syntax"],
        }),
    },
];
#[test]
fn succeeds() {
    check::<BindingEventItem>();
    run(CASES, Word::Succeeds, |input| {
        Ok(proposed::<BindingEventItem>(input))
    });
}
