//! The `parameter` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ParameterItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{
    Reading, want_file, want_name, want_node, want_number, want_span, want_unknown,
};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_receiver_the_parameters_and_the_named_results_are_rows"),
        word: Word::Succeeds,
        input: || "func (t *T) m(a, b int, c ...string) (n int, err error) {}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(8),
                    want_name("receiver"),
                    want_name("receiver"),
                    want_name("t"),
                    want_number(0),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 7), (1, 8)),
                ],
                &[
                    want_node(22),
                    want_name("positional"),
                    want_name("parameter"),
                    want_name("a"),
                    want_number(0),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 15), (1, 16)),
                ],
                &[
                    want_node(26),
                    want_name("positional"),
                    want_name("parameter"),
                    want_name("b"),
                    want_number(1),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 18), (1, 19)),
                ],
                &[
                    want_node(34),
                    want_name("variadic-positional"),
                    want_name("variadic"),
                    want_name("c"),
                    want_number(2),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 25), (1, 26)),
                ],
                &[
                    want_node(46),
                    want_name("result"),
                    want_name("result"),
                    want_name("n"),
                    want_number(0),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 39), (1, 40)),
                ],
                &[
                    want_node(54),
                    want_name("result"),
                    want_name("result"),
                    want_name("err"),
                    want_number(1),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 46), (1, 49)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_parameter_the_declaration_does_not_name_has_the_unknown_value"),
        word: Word::Succeeds,
        input: || "func (T) m(int, string) (int, error) {}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(8),
                    want_name("receiver"),
                    want_name("receiver"),
                    want_unknown("unnamed"),
                    want_number(0),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 7), (1, 8)),
                ],
                &[
                    want_node(17),
                    want_name("positional"),
                    want_name("parameter"),
                    want_unknown("unnamed"),
                    want_number(0),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 12), (1, 15)),
                ],
                &[
                    want_node(22),
                    want_name("positional"),
                    want_name("parameter"),
                    want_unknown("unnamed"),
                    want_number(1),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 17), (1, 23)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_function_literal_has_parameters"),
        word: Word::Succeeds,
        input: || "var f = func(p int) {}\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(16),
                want_name("positional"),
                want_name("parameter"),
                want_name("p"),
                want_number(0),
                want_node(10),
                want_file(SHOWN),
                want_span((1, 14), (1, 15)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("type_parameters_and_the_parameters_of_a_type_are_no_parameters"),
        word: Word::Succeeds,
        input: || "func f[E any](g func(int)) {}\ntype F func(a int)\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(19),
                want_name("positional"),
                want_name("parameter"),
                want_name("g"),
                want_number(0),
                want_node(1),
                want_file(SHOWN),
                want_span((1, 15), (1, 16)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "named_parameters_are_placed_in_their_lists_and_unnamed_ones_have_the_unknown_value_of_unnamed",
        ),
        word: Word::Succeeds,
        input: || "package p\nfunc (r T) f(a, b int, c ...string) (x, y int) {}\nfunc g(int, string) {}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(13),
                    want_name("receiver"),
                    want_name("receiver"),
                    want_name("r"),
                    want_number(0),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 7), (2, 8)),
                ],
                &[
                    want_node(25),
                    want_name("positional"),
                    want_name("parameter"),
                    want_name("a"),
                    want_number(0),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 14), (2, 15)),
                ],
                &[
                    want_node(29),
                    want_name("positional"),
                    want_name("parameter"),
                    want_name("b"),
                    want_number(1),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 17), (2, 18)),
                ],
                &[
                    want_node(37),
                    want_name("variadic-positional"),
                    want_name("variadic"),
                    want_name("c"),
                    want_number(2),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 24), (2, 25)),
                ],
                &[
                    want_node(49),
                    want_name("result"),
                    want_name("result"),
                    want_name("x"),
                    want_number(0),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 38), (2, 39)),
                ],
                &[
                    want_node(53),
                    want_name("result"),
                    want_name("result"),
                    want_name("y"),
                    want_number(1),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 41), (2, 42)),
                ],
                &[
                    want_node(73),
                    want_name("positional"),
                    want_name("parameter"),
                    want_unknown("unnamed"),
                    want_number(0),
                    want_node(64),
                    want_file(SHOWN),
                    want_span((3, 8), (3, 11)),
                ],
                &[
                    want_node(78),
                    want_name("positional"),
                    want_name("parameter"),
                    want_unknown("unnamed"),
                    want_number(1),
                    want_node(64),
                    want_file(SHOWN),
                    want_span((3, 13), (3, 19)),
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
