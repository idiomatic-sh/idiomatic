//! The `syntax_child` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxChildItem;
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
    name: CaseName("every_element_but_the_root_has_a_parent_and_a_value_its_role"),
    word: Word::Succeeds,
    input: || "[1]",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(0),
                want_node(1),
                want_number(0),
                want_name("value"),
                want_file(SHOWN),
            ],
            &[
                want_node(1),
                want_node(2),
                want_number(0),
                want_name("none"),
                want_file(SHOWN),
            ],
            &[
                want_node(1),
                want_node(3),
                want_number(1),
                want_name("value"),
                want_file(SHOWN),
            ],
            &[
                want_node(1),
                want_node(4),
                want_number(2),
                want_name("none"),
                want_file(SHOWN),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<SyntaxChildItem>();
    reading::<SyntaxChildItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
