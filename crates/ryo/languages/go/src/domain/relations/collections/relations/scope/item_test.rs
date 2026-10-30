//! The `scope` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ScopeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_file_and_each_function_are_scopes_under_the_one_that_holds_them"),
        word: Word::Succeeds,
        input: || "func f() { g := func() {} }\nfunc (t T) m() {}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 1), (3, 1)),
                ],
                &[
                    want_node(0),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 28)),
                ],
                &[
                    want_node(0),
                    want_node(33),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 18)),
                ],
                &[
                    want_node(1),
                    want_node(20),
                    want_file(SHOWN),
                    want_span((1, 17), (1, 26)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_file_and_each_function_are_scopes_under_the_one_that_encloses_them"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tg := func() {}\n}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 1), (5, 1)),
                ],
                &[
                    want_node(0),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 1), (4, 2)),
                ],
                &[
                    want_node(6),
                    want_node(25),
                    want_file(SHOWN),
                    want_span((3, 7), (3, 16)),
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
