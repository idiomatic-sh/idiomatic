//! The `sequence_element` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SequenceElementItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{
    Reading, want_file, want_flag, want_name, want_node, want_number, want_span,
};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("values_of_an_array_in_their_places"),
        word: Word::Succeeds,
        input: || "[1,\"a\",[true]]",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_name(""),
                    want_name("boolean"),
                    want_node(9),
                    want_number(0),
                    want_node(7),
                    want_file(SHOWN),
                    want_span((1, 9), (1, 13)),
                    want_name("true"),
                    want_flag(true),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
                &[
                    want_name(""),
                    want_name("integer"),
                    want_node(3),
                    want_number(0),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 2), (1, 3)),
                    want_name("1"),
                    want_flag(false),
                    want_number(1),
                    want_name(""),
                    want_name(""),
                    want_number(10),
                ],
                &[
                    want_name(""),
                    want_name("sequence"),
                    want_node(7),
                    want_number(2),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 8), (1, 14)),
                    want_name("[true]"),
                    want_flag(false),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
                &[
                    want_name("a"),
                    want_name("string"),
                    want_node(5),
                    want_number(1),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 7)),
                    want_name("\"a\""),
                    want_flag(false),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_null_element_is_an_element_of_kind_null"),
        word: Word::Succeeds,
        input: || "[null]",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_name(""),
                want_name("null"),
                want_node(3),
                want_number(0),
                want_node(0),
                want_file(SHOWN),
                want_span((1, 2), (1, 6)),
                want_name("null"),
                want_flag(false),
                want_number(0),
                want_name(""),
                want_name(""),
                want_number(0),
            ]],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<SequenceElementItem>();
    reading::<SequenceElementItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
