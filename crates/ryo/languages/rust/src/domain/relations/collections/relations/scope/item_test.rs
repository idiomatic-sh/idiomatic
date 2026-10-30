//! The `scope` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ScopeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_file_has_its_module_which_encloses_itself"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(0),
                want_node(0),
                want_file(SHOWN),
                want_span((1, 1), (1, 1)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_top_level_function_is_enclosed_by_the_module_and_a_nested_one_by_it"),
        word: Word::Succeeds,
        input: || "fn outer() { fn inner() {} }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 29)),
                ],
                &[
                    want_node(0),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 29)),
                ],
                &[
                    want_node(1),
                    want_node(14),
                    want_file(SHOWN),
                    want_span((1, 14), (1, 27)),
                ],
            ],
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
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
