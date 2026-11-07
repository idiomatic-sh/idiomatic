//! The `function` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::FunctionItem;
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
        name: CaseName("a_named_function_is_a_row"),
        word: Word::Succeeds,
        input: || "fn main() {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name("main"),
                want_file(SHOWN),
                want_span((1, 1), (1, 13)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_unnamed_function_is_not_a_row"),
        word: Word::Succeeds,
        input: || "fn (",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_method_is_a_row_and_a_closure_is_not"),
        word: Word::Succeeds,
        input: || "impl S { fn m(&self) { let c = |x: u8| x; } }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(13),
                want_name("m"),
                want_file(SHOWN),
                want_span((1, 10), (1, 44)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_nested_function_is_a_row_too"),
        word: Word::Succeeds,
        input: || "fn a() { fn b() {} }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 21)),
                ],
                &[
                    want_node(14),
                    want_name("b"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 19)),
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
