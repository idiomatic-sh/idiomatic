//! The `string_literal` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StringLiteralItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{
    Reading, want_file, want_flag, want_name, want_node, want_span, want_unknown,
};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_string_is_a_row_with_the_text_it_stands_for"),
        word: Word::Succeeds,
        input: || "var a = \"x\\ty \\u00e9\"\nvar b = `r\\n`\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(10),
                    want_node(0),
                    want_name("x\ty é"),
                    want_file(SHOWN),
                    want_span((1, 9), (1, 22)),
                    want_flag(false),
                ],
                &[
                    want_node(22),
                    want_node(0),
                    want_name("r\\n"),
                    want_file(SHOWN),
                    want_span((2, 9), (2, 14)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_string_that_is_no_text_has_the_unknown_value"),
        word: Word::Succeeds,
        input: || "var a = \"\\xff\"\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(10),
                want_node(0),
                want_unknown("not-text"),
                want_file(SHOWN),
                want_span((1, 9), (1, 15)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("another_literal_is_no_string"),
        word: Word::Succeeds,
        input: || "var a = 1\nvar b = 'c'\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("strings_are_decoded_and_characters_are_no_strings"),
        word: Word::Succeeds,
        input: || "package p\nvar a = \"x\\ty\"\nvar b = `r\\n`\nvar c = 'c'\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(15),
                    want_node(0),
                    want_name("x\ty"),
                    want_file(SHOWN),
                    want_span((2, 9), (2, 15)),
                    want_flag(false),
                ],
                &[
                    want_node(27),
                    want_node(0),
                    want_name("r\\n"),
                    want_file(SHOWN),
                    want_span((3, 9), (3, 14)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<StringLiteralItem>();
    reading::<StringLiteralItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
