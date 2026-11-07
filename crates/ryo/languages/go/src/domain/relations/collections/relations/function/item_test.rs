//! The `function` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::FunctionItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_declaration_is_a_row_by_its_name"),
        word: Word::Succeeds,
        input: || "func f() {}\nfunc (t T) m() {}\nfunc g[E any]() {}\nfunc h()\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("f"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 12)),
                ],
                &[
                    want_node(15),
                    want_name("m"),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 18)),
                ],
                &[
                    want_node(39),
                    want_name("g"),
                    want_file(SHOWN),
                    want_span((3, 1), (3, 19)),
                ],
                &[
                    want_node(62),
                    want_name("h"),
                    want_file(SHOWN),
                    want_span((4, 1), (4, 9)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_function_literal_is_no_function"),
        word: Word::Succeeds,
        input: || "var f = func() {}\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("declarations_and_methods_are_functions_and_literals_are_not"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {}\nfunc (t T) m() {}\nvar g = func() {}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(6),
                    want_name("f"),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 12)),
                ],
                &[
                    want_node(20),
                    want_name("m"),
                    want_file(SHOWN),
                    want_span((3, 1), (3, 18)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<FunctionItem>();
    reading::<FunctionItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
