//! The `string_literal` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StringLiteralItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("code_is_what_the_library_decoded"),
        word: Word::Succeeds,
        input: || "`` a`b ``\n\n```sh\nx\n```\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_node(1),
                    want_name("a`b"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 10)),
                    want_flag(false),
                ],
                &[
                    want_node(5),
                    want_node(0),
                    want_name("x\n"),
                    want_file(SHOWN),
                    want_span((3, 1), (5, 4)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("indented_code_in_a_quote_is_its_lines_without_the_markers"),
        word: Word::Succeeds,
        input: || ">     x\n>     y\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(3),
                want_node(1),
                want_name("x\ny\n"),
                want_file(SHOWN),
                want_span((1, 7), (3, 1)),
                want_flag(false),
            ]],
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
