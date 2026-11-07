//! Cases for TypeScript assigns rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::AssignsItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("constant_string_assignment"),
        word: Word::Succeeds,
        input: || "const x = \"hi\";",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(4),
                want_node(5),
                want_node(10),
                want_node(0),
                want_file("item.ts"),
                want_span((1, 11), (1, 15)),
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
    check::<AssignsItem>();
    reading::<AssignsItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
