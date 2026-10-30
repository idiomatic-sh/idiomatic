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
        name: CaseName("every_html_comment_is_a_row_with_its_text_between_the_delimiters"),
        word: Word::Succeeds,
        input: || "<!-- a -->\n\nb <!-- c -->\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name(" a "),
                    want_file(SHOWN),
                    want_span((1, 1), (2, 1)),
                    want_flag(false),
                ],
                &[
                    want_node(6),
                    want_name(" c "),
                    want_file(SHOWN),
                    want_span((3, 3), (3, 13)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("other_html_is_no_comment"),
        word: Word::Succeeds,
        input: || "<div>\n\na <b>c</b>\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_comment_in_code_is_code"),
        word: Word::Succeeds,
        input: || "`<!-- a -->`\n",
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
