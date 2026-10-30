//! The `assigns` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::AssignsItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_string_literal_is_assigned"),
        word: Word::Succeeds,
        input: || "fn run() { let value = \"ready\"; }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(23),
                want_node(18),
                want_node(23),
                want_node(1),
                want_file(SHOWN),
                want_span((1, 24), (1, 31)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_computed_value_is_not"),
        word: Word::Succeeds,
        input: || "fn run() { let value = make(); }",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<AssignsItem>();
    reading::<AssignsItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
