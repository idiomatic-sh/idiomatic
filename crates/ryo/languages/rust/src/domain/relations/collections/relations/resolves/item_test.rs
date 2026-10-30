//! The `resolves` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ResolvesItem;
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
        name: CaseName("a_type_declared_in_the_file_is_a_row"),
        word: Word::Succeeds,
        input: || "struct A { b: B }\nstruct B;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(15),
                want_node(1),
                want_name("B"),
                want_node(23),
                want_file(SHOWN),
                want_span((1, 15), (1, 16)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "an_import_a_type_parameter_and_a_wrapped_type_are_rows_and_a_qualified_path_is_not",
        ),
        word: Word::Succeeds,
        input: || "use crate::x::Foo;\nstruct A<T> { a: Foo, t: T, v: Vec<Foo>, q: x::Foo }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(41),
                    want_node(21),
                    want_name("Foo"),
                    want_node(4),
                    want_file(SHOWN),
                    want_span((2, 18), (2, 21)),
                ],
                &[
                    want_node(53),
                    want_node(21),
                    want_name("T"),
                    want_node(28),
                    want_file(SHOWN),
                    want_span((2, 26), (2, 27)),
                ],
                &[
                    want_node(73),
                    want_node(21),
                    want_name("Foo"),
                    want_node(4),
                    want_file(SHOWN),
                    want_span((2, 36), (2, 39)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<ResolvesItem>();
    reading::<ResolvesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
