//! The `string_literal` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StringLiteralItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("string_values_are_literals_and_keys_are_not"),
    word: Word::Succeeds,
    input: || "{\"k\":\"v\",\"l\":[\"w\"]}",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(6),
                want_node(3),
                want_name("v"),
                want_file(SHOWN),
                want_span((1, 6), (1, 9)),
                want_flag(false),
            ],
            &[
                want_node(13),
                want_node(11),
                want_name("w"),
                want_file(SHOWN),
                want_span((1, 15), (1, 18)),
                want_flag(false),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<StringLiteralItem>();
    reading::<StringLiteralItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
