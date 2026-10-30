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
    name: CaseName(
        "a_declared_type_is_bound_behind_a_pointer_or_a_slice_and_a_qualified_or_unknown_type_is_not",
    ),
    word: Word::Succeeds,
    input: || "package p\n\nimport \"m\"\n\ntype Local struct{}\n\ntype Stored struct {\n\trecord m.Record\n\titems  []Local\n\tlocal  *Local\n\tother  Missing\n}\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(57),
                want_node(29),
                want_name("Local"),
                want_node(16),
                want_file(SHOWN),
                want_span((9, 11), (9, 16)),
            ],
            &[
                want_node(66),
                want_node(29),
                want_name("Local"),
                want_node(16),
                want_file(SHOWN),
                want_span((10, 10), (10, 15)),
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
