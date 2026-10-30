//! The `annotates` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::AnnotatesItem;
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
    name: CaseName("parameters_results_and_targets_are_annotated"),
    word: Word::Succeeds,
    input: || "def f(a: int, b: \"Fwd\" = 1) -> list[int]:\n    x: str = \"s\"\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(14),
                want_node(9),
                want_name("int"),
                want_file(SHOWN),
                want_span((1, 10), (1, 13)),
            ],
            &[
                want_node(24),
                want_node(19),
                want_name("\"Fwd\""),
                want_file(SHOWN),
                want_span((1, 18), (1, 23)),
            ],
            &[
                want_node(36),
                want_node(1),
                want_name("list[int]"),
                want_file(SHOWN),
                want_span((1, 32), (1, 41)),
            ],
            &[
                want_node(51),
                want_node(47),
                want_name("str"),
                want_file(SHOWN),
                want_span((2, 8), (2, 11)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<AnnotatesItem>();
    reading::<AnnotatesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
