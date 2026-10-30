//! The `comment` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::CommentItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN_COMMENTED, reading_commented};
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_line_and_a_block_comment"),
        word: Word::Succeeds,
        input: || "{ // c\n\"a\":/* x */1}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(4),
                    want_name(" c"),
                    want_file(SHOWN_COMMENTED),
                    want_span((1, 3), (1, 7)),
                    want_flag(false),
                ],
                &[
                    want_node(9),
                    want_name(" x "),
                    want_file(SHOWN_COMMENTED),
                    want_span((2, 5), (2, 12)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_slash_in_a_string_is_no_comment"),
        word: Word::Succeeds,
        input: || "{\"a\":\"// no\"}",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<CommentItem>();
    reading_commented::<CommentItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
