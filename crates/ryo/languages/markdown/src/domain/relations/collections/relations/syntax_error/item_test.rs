//! The `syntax_error` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxErrorItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading_bytes};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static [u8];
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("bytes_that_are_not_text_are_reported"),
        word: Word::Succeeds,
        input: || b"# A\n\xff\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name(
                    "invalid UTF-8: a Markdown file is UTF-8 text, and the parser reads it no further",
                ),
                want_file(SHOWN),
                want_span((2, 1), (3, 1)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_unclosed_fence_is_no_error"),
        word: Word::Succeeds,
        input: || b"```\nx\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<SyntaxErrorItem>();
    reading_bytes::<SyntaxErrorItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
