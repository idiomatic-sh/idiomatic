//! The `mapping` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::MappingItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_root_object_a_member_value_and_an_array_element"),
    word: Word::Succeeds,
    input: || "{\"a\":{\"b\":1},\"c\":[{}]}",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(0),
                want_name("standard"),
                want_number(0),
                want_node(0),
                want_file(SHOWN),
            ],
            &[
                want_node(6),
                want_name("inline"),
                want_number(0),
                want_node(4),
                want_file(SHOWN),
            ],
            &[
                want_node(19),
                want_name("sequence-member"),
                want_number(0),
                want_node(19),
                want_file(SHOWN),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<MappingItem>();
    reading::<MappingItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
