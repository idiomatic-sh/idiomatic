//! The `defines` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::DefinesItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("headings_are_sections_and_definitions_are_labels"),
    word: Word::Succeeds,
    input: || "# Title\n\n## Sub *x*\n\ntext <a id=\"anchor\"></a> and [ref]\n\n[ref]: guide.md\n\nnote[^1]\n\n[^1]: a note\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(2),
                want_name("section"),
                want_name("heading"),
                want_name("Title"),
                want_node(0),
                want_file(SHOWN),
                want_span((1, 1), (2, 1)),
            ],
            &[
                want_node(8),
                want_name("section"),
                want_name("heading"),
                want_name("Sub x"),
                want_node(2),
                want_file(SHOWN),
                want_span((3, 1), (4, 1)),
            ],
            &[
                want_node(19),
                want_name("label"),
                want_name("html"),
                want_name("anchor"),
                want_node(8),
                want_file(SHOWN),
                want_span((5, 6), (5, 21)),
            ],
            &[
                want_node(28),
                want_name("label"),
                want_name("definition"),
                want_name("ref"),
                want_node(8),
                want_file(SHOWN),
                want_span((7, 1), (7, 16)),
            ],
            &[
                want_node(35),
                want_name("label"),
                want_name("footnote-definition"),
                want_name("1"),
                want_node(8),
                want_file(SHOWN),
                want_span((11, 1), (12, 1)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<DefinesItem>();
    reading::<DefinesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
