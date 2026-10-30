//! The `comment` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::CommentItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_comment_is_a_row_with_its_text_after_the_hash"),
        word: Word::Succeeds,
        input: || "# first\na = 1 # trailing\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name(" first"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 8)),
                    want_flag(false),
                ],
                &[
                    want_node(11),
                    want_name(" trailing"),
                    want_file(SHOWN),
                    want_span((2, 7), (2, 17)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_hash_in_a_string_is_no_comment"),
        word: Word::Succeeds,
        input: || "a = \"# no\"\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<CommentItem>();
    reading::<CommentItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
