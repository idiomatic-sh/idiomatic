//! assigns reader cases and the collection contract.
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
        name: CaseName("a_declarator_and_an_assignment_bind_strings"),
        word: Word::Succeeds,
        input: || "const a = \"x\";\nb = \"y\";\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(4),
                    want_node(5),
                    want_node(10),
                    want_node(0),
                    want_file("item.js"),
                    want_span((1, 11), (1, 14)),
                ],
                &[
                    want_node(15),
                    want_node(16),
                    want_node(21),
                    want_node(0),
                    want_file("item.js"),
                    want_span((2, 5), (2, 8)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("empty_source_has_no_assigns"),
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
