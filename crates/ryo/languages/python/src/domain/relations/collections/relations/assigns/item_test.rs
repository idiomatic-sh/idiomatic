//! The `assigns` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::AssignsItem;
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
    name: CaseName("a_single_name_bound_to_a_known_string_is_an_assignment"),
    word: Word::Succeeds,
    input: || "a = b = \"s\"\nc: str = \"t\"\n(d := \"u\")\ne = 1\nf, g = \"v\"\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(1),
                want_node(2),
                want_node(12),
                want_node(0),
                want_file(SHOWN),
                want_span((1, 9), (1, 12)),
            ],
            &[
                want_node(1),
                want_node(7),
                want_node(12),
                want_node(0),
                want_file(SHOWN),
                want_span((1, 9), (1, 12)),
            ],
            &[
                want_node(16),
                want_node(17),
                want_node(26),
                want_node(0),
                want_file(SHOWN),
                want_span((2, 10), (2, 13)),
            ],
            &[
                want_node(32),
                want_node(33),
                want_node(38),
                want_node(0),
                want_file(SHOWN),
                want_span((3, 7), (3, 10)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<AssignsItem>();
    reading::<AssignsItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
