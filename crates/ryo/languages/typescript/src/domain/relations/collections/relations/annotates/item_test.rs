//! Cases for TypeScript annotates rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::AnnotatesItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("type_annotation"),
        word: Word::Succeeds,
        input: || "let x: string;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(7),
                want_node(5),
                want_name("string"),
                want_file("item.ts"),
                want_span((1, 8), (1, 14)),
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
    check::<AnnotatesItem>();
    reading::<AnnotatesItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
