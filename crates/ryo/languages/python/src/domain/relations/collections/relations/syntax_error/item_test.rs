//! The `syntax_error` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxErrorItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_broken_definition_is_reported"),
        word: Word::Succeeds,
        input: || "def broken(\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("unexpected EOF while parsing"),
                want_file(SHOWN),
                want_span((2, 1), (2, 1)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_else_with_no_except_is_an_error_as_cpython_says"),
        word: Word::Succeeds,
        input: || "try:\n    pass\nelse:\n    pass\nfinally:\n    pass\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("expected 'except' or 'finally' block"),
                want_file(SHOWN),
                want_span((3, 1), (3, 5)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_unknown_future_feature_is_an_error_and_braces_not_a_chance"),
        word: Word::Succeeds,
        input: || "from __future__ import annotations, nope\nfrom __future__ import braces\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2_147_483_648),
                    want_name("future feature nope is not defined"),
                    want_file(SHOWN),
                    want_span((1, 37), (1, 41)),
                ],
                &[
                    want_node(2_147_483_649),
                    want_name("not a chance"),
                    want_file(SHOWN),
                    want_span((2, 24), (2, 30)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<SyntaxErrorItem>();
    reading::<SyntaxErrorItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
