//! The `comment` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::CommentItem;
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
        name: CaseName("every_comment_is_a_row_with_its_text_after_the_hash"),
        word: Word::Succeeds,
        input: || "# first\nx = 1  # trailing\n",
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
                    want_node(12),
                    want_name(" trailing"),
                    want_file(SHOWN),
                    want_span((2, 8), (2, 18)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_hash_in_a_string_is_no_comment"),
        word: Word::Succeeds,
        input: || "x = \"# no\"\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("comment_survives_target_syntax_error_with_complete_tokens"),
        word: Word::Succeeds,
        input: || "value = ur\"text\" # retained\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(14),
                want_name(" retained"),
                want_file(SHOWN),
                want_span((1, 18), (1, 28)),
                want_flag(false),
            ]],
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
