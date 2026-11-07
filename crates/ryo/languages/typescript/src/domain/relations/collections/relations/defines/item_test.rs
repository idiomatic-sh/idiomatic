//! Cases for TypeScript defines rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::DefinesItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("constant_binding"),
        word: Word::Succeeds,
        input: || "const x = 1;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(4),
                want_name("constant"),
                want_name("variable-declarator"),
                want_name("x"),
                want_node(0),
                want_file("item.ts"),
                want_span((1, 7), (1, 8)),
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
    check::<DefinesItem>();
    reading::<DefinesItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
