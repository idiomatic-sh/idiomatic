//! The `inherits` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::InheritsItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("embedded_types_and_interfaces_are_bases_and_constraints_are_not"),
    word: Word::Succeeds,
    input: || "package p\ntype S struct {\n\tBase\n\t*pkg.Other[int]\n\tname string\n}\ntype I interface {\n\tio.Reader\n\t~int | string\n\tm()\n}\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(20),
                want_node(9),
                want_name("Base"),
                want_file(SHOWN),
                want_span((3, 2), (3, 6)),
            ],
            &[
                want_node(24),
                want_node(9),
                want_name("pkg.Other"),
                want_file(SHOWN),
                want_span((4, 2), (4, 17)),
            ],
            &[
                want_node(61),
                want_node(50),
                want_name("io.Reader"),
                want_file(SHOWN),
                want_span((8, 2), (8, 11)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<InheritsItem>();
    reading::<InheritsItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
