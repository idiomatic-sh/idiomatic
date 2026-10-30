//! reference reader cases and the collection contract.
use super::super::fixtures::contract::check;
use super::item::ReferenceItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_member_chain_is_one_reference_and_an_argument_another"),
        word: Word::Succeeds,
        input: || "a.b.c(d);\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(3),
                    want_node(0),
                    want_name("a.b.c"),
                    want_file("item.js"),
                    want_span((1, 1), (1, 6)),
                ],
                &[
                    want_node(14),
                    want_node(0),
                    want_name("d"),
                    want_file("item.js"),
                    want_span((1, 7), (1, 8)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("empty_source_has_no_reference"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<ReferenceItem>();
    reading::<ReferenceItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
