//! inherits reader cases and the collection contract.
use super::super::fixtures::contract::check;
use super::item::InheritsItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_class_inherits_what_it_extends"),
        word: Word::Succeeds,
        input: || "class A extends B.C {}\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(9),
                want_node(1),
                want_name("B.C"),
                want_file("item.js"),
                want_span((1, 17), (1, 20)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("empty_source_has_no_inherits"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<InheritsItem>();
    reading::<InheritsItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
