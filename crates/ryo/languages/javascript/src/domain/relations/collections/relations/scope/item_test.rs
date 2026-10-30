//! scope reader cases and the collection contract.
use super::super::fixtures::contract::check;
use super::item::ScopeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_function_and_its_blocks_nest"),
        word: Word::Succeeds,
        input: || "function f() { { } }\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_node(0),
                    want_file("item.js"),
                    want_span((1, 1), (2, 1)),
                ],
                &[
                    want_node(0),
                    want_node(1),
                    want_file("item.js"),
                    want_span((1, 1), (1, 21)),
                ],
                &[
                    want_node(1),
                    want_node(9),
                    want_file("item.js"),
                    want_span((1, 14), (1, 21)),
                ],
                &[
                    want_node(9),
                    want_node(12),
                    want_file("item.js"),
                    want_span((1, 16), (1, 19)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_empty_file_is_one_scope"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(0),
                want_node(0),
                want_file("item.js"),
                want_span((1, 1), (1, 1)),
            ]],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<ScopeItem>();
    reading::<ScopeItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
