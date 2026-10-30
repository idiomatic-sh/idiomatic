//! The `defines` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::DefinesItem;
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
        name: CaseName("the_names_a_destructuring_let_binds_are_variables"),
        word: Word::Succeeds,
        input: || "fn run() { let (a, (b, c)) = (1, (2, 3)); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("function"),
                    want_name("function"),
                    want_name("run"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 7)),
                ],
                &[
                    want_node(20),
                    want_name("variable"),
                    want_name("variable"),
                    want_name("a"),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 17), (1, 18)),
                ],
                &[
                    want_node(27),
                    want_name("variable"),
                    want_name("variable"),
                    want_name("b"),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 21), (1, 22)),
                ],
                &[
                    want_node(32),
                    want_name("variable"),
                    want_name("variable"),
                    want_name("c"),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 24), (1, 25)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_function_and_a_binding_are_declarations"),
        word: Word::Succeeds,
        input: || "fn run() { let x = 1; }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("function"),
                    want_name("function"),
                    want_name("run"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 7)),
                ],
                &[
                    want_node(18),
                    want_name("variable"),
                    want_name("variable"),
                    want_name("x"),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 16), (1, 17)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_label_is_a_declaration"),
        word: Word::Succeeds,
        input: || "fn spin() { 'outer: loop { break 'outer; } }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("function"),
                    want_name("function"),
                    want_name("spin"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 8)),
                ],
                &[
                    want_node(15),
                    want_name("label"),
                    want_name("label"),
                    want_name("'outer"),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 13), (1, 19)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_struct_and_its_positional_fields_are_declarations"),
        word: Word::Succeeds,
        input: || "struct S(u8, u8);",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("struct"),
                    want_name("struct"),
                    want_name("S"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 8), (1, 9)),
                ],
                &[
                    want_node(8),
                    want_name("property"),
                    want_name("tuple-field"),
                    want_name("0"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 12)),
                ],
                &[
                    want_node(16),
                    want_name("property"),
                    want_name("tuple-field"),
                    want_name("1"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 14), (1, 16)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("each_leaf_of_a_use_binds_a_name"),
        word: Word::Succeeds,
        input: || "use std::{io::{self, Write}, fmt::*};",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(20),
                    want_name("import"),
                    want_name("use"),
                    want_name("io"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 11), (1, 13)),
                ],
                &[
                    want_node(27),
                    want_name("import"),
                    want_name("use"),
                    want_name("Write"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 22), (1, 27)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_name_that_starts_as_a_constant_does_may_be_one"),
        word: Word::Succeeds,
        input: || "fn run() { let (Unit, b) = (1, 2); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("function"),
                    want_name("function"),
                    want_name("run"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 7)),
                ],
                &[
                    want_node(25),
                    want_name("variable"),
                    want_name("variable"),
                    want_name("b"),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 23), (1, 24)),
                ],
            ],
            causes: &[
                "a name in a destructuring `let` that starts as a constant does may be one, and not a variable",
            ],
        }),
    },
    Case {
        name: CaseName("none_is_the_variant_and_not_a_name_a_pattern_binds"),
        word: Word::Succeeds,
        input: || "fn run() { let (None, b) = (1, 2); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("function"),
                    want_name("function"),
                    want_name("run"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 7)),
                ],
                &[
                    want_node(25),
                    want_name("variable"),
                    want_name("variable"),
                    want_name("b"),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 23), (1, 24)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("lowercase_names_in_a_pattern_are_variables_and_read_whole"),
        word: Word::Succeeds,
        input: || "fn run() { let (a, b) = (1, 2); let _ = 3; }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("function"),
                    want_name("function"),
                    want_name("run"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 7)),
                ],
                &[
                    want_node(20),
                    want_name("variable"),
                    want_name("variable"),
                    want_name("a"),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 17), (1, 18)),
                ],
                &[
                    want_node(25),
                    want_name("variable"),
                    want_name("variable"),
                    want_name("b"),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 20), (1, 21)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_impl_is_named_by_its_header"),
        word: Word::Succeeds,
        input: || "impl<T: Clone> Display for Point<T> where T: Copy {}\nimpl Point {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("implementation"),
                    want_name("impl"),
                    want_name("Display for Point"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 51)),
                ],
                &[
                    want_node(5),
                    want_name("type-parameter"),
                    want_name("type-parameter"),
                    want_name("T"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 6), (1, 7)),
                ],
                &[
                    want_node(65),
                    want_name("implementation"),
                    want_name("impl"),
                    want_name("Point"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 12)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<DefinesItem>();
    reading::<DefinesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
