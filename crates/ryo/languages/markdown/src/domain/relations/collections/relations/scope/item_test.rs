//! The `scope` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ScopeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_document_encloses_itself_and_sections_nest"),
    word: Word::Succeeds,
    input: || "x\n\n# a\n\n## b\n\n# c\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(0),
                want_node(0),
                want_file(SHOWN),
                want_span((1, 1), (8, 1)),
            ],
            &[
                want_node(0),
                want_node(6),
                want_file(SHOWN),
                want_span((3, 1), (7, 1)),
            ],
            &[
                want_node(0),
                want_node(18),
                want_file(SHOWN),
                want_span((7, 1), (8, 1)),
            ],
            &[
                want_node(6),
                want_node(12),
                want_file(SHOWN),
                want_span((5, 1), (7, 1)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<ScopeItem>();
    reading::<ScopeItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
