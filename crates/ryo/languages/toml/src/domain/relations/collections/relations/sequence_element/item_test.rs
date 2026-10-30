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
        name: CaseName("an_array_of_tables_gives_the_rows_an_inline_array_does"),
        word: Word::Succeeds,
        input: || "[[bin]]\n[[bin]]\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_name(""),
                    want_name("mapping"),
                    want_node(1),
                    want_number(0),
                    want_node(4),
                    want_file(SHOWN),
                    want_span((1, 1), (2, 1)),
                    want_name("[[bin]]\n"),
                    want_flag(false),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
                &[
                    want_name(""),
                    want_name("mapping"),
                    want_node(7),
                    want_number(1),
                    want_node(4),
                    want_file(SHOWN),
                    want_span((2, 1), (3, 1)),
                    want_name("[[bin]]\n"),
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
        name: CaseName("an_inline_array_of_tables"),
        word: Word::Succeeds,
        input: || "bin = [{}, {}]\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_name(""),
                    want_name("inline-mapping"),
                    want_node(9),
                    want_number(0),
                    want_node(3),
                    want_file(SHOWN),
                    want_span((1, 8), (1, 10)),
                    want_name("{}"),
                    want_flag(false),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
                &[
                    want_name(""),
                    want_name("inline-mapping"),
                    want_node(14),
                    want_number(1),
                    want_node(3),
                    want_file(SHOWN),
                    want_span((1, 12), (1, 14)),
                    want_name("{}"),
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
        name: CaseName("nested_arrays_are_held_by_their_element"),
        word: Word::Succeeds,
        input: || "a = [1, [2, \"x\"]]\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_name(""),
                    want_name("integer"),
                    want_node(9),
                    want_number(0),
                    want_node(3),
                    want_file(SHOWN),
                    want_span((1, 6), (1, 7)),
                    want_name("1"),
                    want_flag(false),
                    want_number(1),
                    want_name(""),
                    want_name(""),
                    want_number(10),
                ],
                &[
                    want_name(""),
                    want_name("integer"),
                    want_node(14),
                    want_number(0),
                    want_node(12),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 11)),
                    want_name("2"),
                    want_flag(false),
                    want_number(2),
                    want_name(""),
                    want_name(""),
                    want_number(10),
                ],
                &[
                    want_name(""),
                    want_name("sequence"),
                    want_node(12),
                    want_number(1),
                    want_node(3),
                    want_file(SHOWN),
                    want_span((1, 9), (1, 17)),
                    want_name("[2, \"x\"]"),
                    want_flag(false),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
                &[
                    want_name("x"),
                    want_name("string"),
                    want_node(17),
                    want_number(1),
                    want_node(12),
                    want_file(SHOWN),
                    want_span((1, 13), (1, 16)),
                    want_name("\"x\""),
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
        name: CaseName("a_cargo_lock"),
        word: Word::Succeeds,
        input: || include_str!("../../../../../../tests/fixtures/allocation/cargo-lock.toml"),
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_name(""),
                    want_name("mapping"),
                    want_node(12),
                    want_number(0),
                    want_node(15),
                    want_file(SHOWN),
                    want_span((4, 1), (9, 1)),
                    want_name(
                        "[[package]]\nname = \"anyhow\"\nversion = \"1.0.98\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n",
                    ),
                    want_flag(false),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
                &[
                    want_name(""),
                    want_name("mapping"),
                    want_node(43),
                    want_number(1),
                    want_node(15),
                    want_file(SHOWN),
                    want_span((9, 1), (15, 1)),
                    want_name(
                        "[[package]]\nname = \"demo\"\nversion = \"0.1.0\"\ndependencies = [\n \"anyhow\",\n]\n",
                    ),
                    want_flag(false),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
                &[
                    want_name("anyhow"),
                    want_name("string"),
                    want_node(75),
                    want_number(0),
                    want_node(67),
                    want_file(SHOWN),
                    want_span((13, 2), (13, 10)),
                    want_name("\"anyhow\""),
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
];

fn subject(source: Input) -> Value {
    check::<SequenceElementItem>();
    reading::<SequenceElementItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
