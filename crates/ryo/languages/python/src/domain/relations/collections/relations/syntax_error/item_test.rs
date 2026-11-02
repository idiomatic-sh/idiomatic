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
                want_name("parse"),
                want_name("incomplete"),
                want_name("parser-error"),
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
                want_name("parse"),
                want_name("lexically-complete"),
                want_name("else-without-except"),
                want_file(SHOWN),
                want_span((3, 1), (3, 5)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("module_nonlocal_validation_is_preserved_with_stable_code"),
        word: Word::Succeeds,
        input: || "global x\nnonlocal x\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("nonlocal declaration not allowed at module level"),
                want_name("validation"),
                want_name("preserved"),
                want_name("nonlocal-at-module"),
                want_file(SHOWN),
                want_span((2, 1), (2, 11)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("late_future_directive_remains_validation_on_a_complete_ast"),
        word: Word::Succeeds,
        input: || "x = 1\nfrom __future__ import annotations\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("from __future__ imports must occur at the beginning of the file"),
                want_name("validation"),
                want_name("preserved"),
                want_name("future-directive-invalid"),
                want_file(SHOWN),
                want_span((2, 1), (2, 35)),
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
                    want_name("validation"),
                    want_name("preserved"),
                    want_name("future-feature-not-defined"),
                    want_file(SHOWN),
                    want_span((1, 37), (1, 41)),
                ],
                &[
                    want_node(2_147_483_649),
                    want_name("not a chance"),
                    want_name("validation"),
                    want_name("preserved"),
                    want_name("future-feature-not-defined"),
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
