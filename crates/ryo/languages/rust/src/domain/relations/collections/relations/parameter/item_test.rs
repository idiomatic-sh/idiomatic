//! The `parameter` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ParameterItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_parameter_is_a_row"),
        word: Word::Succeeds,
        input: || "fn run(value: u8) {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(10),
                want_name("positional"),
                want_name("positional"),
                want_name("value"),
                want_number(0),
                want_node(1),
                want_file(SHOWN),
                want_span((1, 8), (1, 17)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_c_variadic_of_a_foreign_function_takes_the_rest"),
        word: Word::Succeeds,
        input: || "unsafe extern \"C\" { fn log(format: u8, ...); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(21),
                    want_name("positional"),
                    want_name("positional"),
                    want_name("format"),
                    want_number(0),
                    want_node(12),
                    want_file(SHOWN),
                    want_span((1, 28), (1, 38)),
                ],
                &[
                    want_node(32),
                    want_name("variadic-positional"),
                    want_name("variadic"),
                    want_name(""),
                    want_number(1),
                    want_node(12),
                    want_file(SHOWN),
                    want_span((1, 40), (1, 43)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_receiver_a_named_parameter_and_a_pattern_are_rows"),
        word: Word::Succeeds,
        input: || "impl S { fn run(&self, value: u8, (a, b): (u8, u8)) {} }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(20),
                    want_name("receiver"),
                    want_name("receiver"),
                    want_name("self"),
                    want_number(0),
                    want_node(13),
                    want_file(SHOWN),
                    want_span((1, 17), (1, 22)),
                ],
                &[
                    want_node(28),
                    want_name("positional"),
                    want_name("positional"),
                    want_name("value"),
                    want_number(1),
                    want_node(13),
                    want_file(SHOWN),
                    want_span((1, 24), (1, 33)),
                ],
                &[
                    want_node(39),
                    want_name("positional"),
                    want_name("positional"),
                    want_name(""),
                    want_number(2),
                    want_node(13),
                    want_file(SHOWN),
                    want_span((1, 35), (1, 51)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_wildcard_is_a_parameter_with_no_name"),
        word: Word::Succeeds,
        input: || "fn run(_: u8, value: u8) {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(8),
                    want_name("positional"),
                    want_name("positional"),
                    want_name(""),
                    want_number(0),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 8), (1, 13)),
                ],
                &[
                    want_node(22),
                    want_name("positional"),
                    want_name("positional"),
                    want_name("value"),
                    want_number(1),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 15), (1, 24)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("closure_parameters_are_rows"),
        word: Word::Succeeds,
        input: || "fn run() { let add = |left, right: u8| left; }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(28),
                    want_name("positional"),
                    want_name("positional"),
                    want_name("left"),
                    want_number(0),
                    want_node(23),
                    want_file(SHOWN),
                    want_span((1, 23), (1, 27)),
                ],
                &[
                    want_node(34),
                    want_name("positional"),
                    want_name("positional"),
                    want_name("right"),
                    want_number(1),
                    want_node(23),
                    want_file(SHOWN),
                    want_span((1, 29), (1, 38)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<ParameterItem>();
    reading::<ParameterItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
