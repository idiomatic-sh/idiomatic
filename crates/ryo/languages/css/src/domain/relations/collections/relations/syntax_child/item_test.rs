//! The `syntax_child` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxChildItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_edge_is_ordered_under_its_parent"),
    word: Word::Succeeds,
    input: || "a{b:c}",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(0),
                want_node(1),
                want_number(0),
                want_name("none"),
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
                want_name("none"),
                want_file(SHOWN),
            ],
            &[
                want_node(1),
                want_node(4),
                want_number(2),
                want_name("none"),
                want_file(SHOWN),
            ],
            &[
                want_node(1),
                want_node(8),
                want_number(3),
                want_name("none"),
                want_file(SHOWN),
            ],
            &[
                want_node(4),
                want_node(5),
                want_number(0),
                want_name("name"),
                want_file(SHOWN),
            ],
            &[
                want_node(4),
                want_node(6),
                want_number(1),
                want_name("none"),
                want_file(SHOWN),
            ],
            &[
                want_node(4),
                want_node(7),
                want_number(2),
                want_name("value"),
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
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
