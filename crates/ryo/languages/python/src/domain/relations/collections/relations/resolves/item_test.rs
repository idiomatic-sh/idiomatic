//! The `resolves` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ResolvesItem;
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
    name: CaseName("an_import_and_a_class_are_bound_and_a_qualified_or_unknown_name_is_not"),
    word: Word::Succeeds,
    input: || "from m import Record\n\nclass Local:\n    pass\n\nclass Stored:\n    record: Record\n    items: list[Record]\n    local: Local | None\n    other: Missing\n    q: m.Record\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(40),
                want_node(27),
                want_name("Record"),
                want_node(9),
                want_file(SHOWN),
                want_span((7, 13), (7, 19)),
            ],
            &[
                want_node(53),
                want_node(27),
                want_name("Record"),
                want_node(9),
                want_file(SHOWN),
                want_span((8, 17), (8, 23)),
            ],
            &[
                want_node(64),
                want_node(27),
                want_name("Local"),
                want_node(14),
                want_file(SHOWN),
                want_span((9, 12), (9, 17)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<ResolvesItem>();
    reading::<ResolvesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
