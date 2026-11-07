//! Cases for TypeScript function rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::FunctionItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("named_functions"),
        word: Word::Succeeds,
        input: || "function f() {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name("f"),
                want_file("item.ts"),
                want_span((1, 1), (1, 16)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_empty_file_has_no_row"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<FunctionItem>();
    reading::<FunctionItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
