//! The `mention` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::MentionItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_identifier_in_the_tokens_of_an_unknown_macro_is_a_mention"),
        word: Word::Succeeds,
        input: || "fn run() { unknown!(a, b); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(24),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 21), (1, 22)),
                ],
                &[
                    want_node(27),
                    want_node(1),
                    want_name("b"),
                    want_file(SHOWN),
                    want_span((1, 24), (1, 25)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_name_of_the_macro_is_no_mention"),
        word: Word::Succeeds,
        input: || "fn run() { unknown!(); }",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_identifiers_of_a_macro_read_as_expressions_are_mentions_too"),
        word: Word::Succeeds,
        input: || "fn run() { println!(\"{}\", a); }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(27),
                want_node(1),
                want_name("a"),
                want_file(SHOWN),
                want_span((1, 27), (1, 28)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_identifier_outside_a_macro_is_no_mention"),
        word: Word::Succeeds,
        input: || "fn run() { let a = 1; a; }",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_identifiers_of_a_macro_rules_body_are_mentions"),
        word: Word::Succeeds,
        input: || "macro_rules! twice { ($x:expr) => { $x + $x }; }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(14),
                    want_node(0),
                    want_name("x"),
                    want_file(SHOWN),
                    want_span((1, 24), (1, 25)),
                ],
                &[
                    want_node(16),
                    want_node(0),
                    want_name("expr"),
                    want_file(SHOWN),
                    want_span((1, 26), (1, 30)),
                ],
                &[
                    want_node(26),
                    want_node(0),
                    want_name("x"),
                    want_file(SHOWN),
                    want_span((1, 38), (1, 39)),
                ],
                &[
                    want_node(31),
                    want_node(0),
                    want_name("x"),
                    want_file(SHOWN),
                    want_span((1, 43), (1, 44)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<MentionItem>();
    reading::<MentionItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
