//! The `annotates` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::AnnotatesItem;
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
        name: CaseName("a_type_on_a_pattern_annotates_the_pattern"),
        word: Word::Succeeds,
        input: || "fn run((a, b): (u8, u8)) { let (c, d): (u8, u8) = (1, 2); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(22),
                    want_node(8),
                    want_name("(u8, u8)"),
                    want_file(SHOWN),
                    want_span((1, 16), (1, 24)),
                ],
                &[
                    want_node(59),
                    want_node(46),
                    want_name("(u8, u8)"),
                    want_file(SHOWN),
                    want_span((1, 40), (1, 48)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("parameters_and_results_are_annotated"),
        word: Word::Succeeds,
        input: || "fn run(count: u8) -> bool { let (a, b) = (1, 2); true }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(14),
                    want_node(10),
                    want_name("u8"),
                    want_file(SHOWN),
                    want_span((1, 15), (1, 17)),
                ],
                &[
                    want_node(24),
                    want_node(1),
                    want_name("bool"),
                    want_file(SHOWN),
                    want_span((1, 22), (1, 26)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_let_with_a_type_is_annotated"),
        word: Word::Succeeds,
        input: || "fn run() { let value: crate::Value = 1; }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(22),
                want_node(18),
                want_name("crate::Value"),
                want_file(SHOWN),
                want_span((1, 23), (1, 35)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_destructured_parameter_is_annotated_as_a_whole"),
        word: Word::Succeeds,
        input: || "fn run(a: u8, (b, c): (u8, u8)) {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(14),
                    want_node(10),
                    want_name("u8"),
                    want_file(SHOWN),
                    want_span((1, 11), (1, 13)),
                ],
                &[
                    want_node(35),
                    want_node(21),
                    want_name("(u8, u8)"),
                    want_file(SHOWN),
                    want_span((1, 23), (1, 31)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_const_and_a_static_are_annotated_with_their_types"),
        word: Word::Succeeds,
        input: || "const LIMIT: u8 = 1;\nstatic COUNT: crate::Count = 0;",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(8),
                    want_node(1),
                    want_name("u8"),
                    want_file(SHOWN),
                    want_span((1, 14), (1, 16)),
                ],
                &[
                    want_node(27),
                    want_node(20),
                    want_name("crate::Count"),
                    want_file(SHOWN),
                    want_span((2, 15), (2, 27)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<AnnotatesItem>();
    reading::<AnnotatesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
