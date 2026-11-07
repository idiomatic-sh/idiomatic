//! The `prose` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ProseItem;
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
    name: CaseName("runs_are_joined_and_code_is_masked"),
    word: Word::Succeeds,
    input: || "# Title *x*\n\nA `code` word <b>raw</b> end.\n\n- item one\n- item **two**\n\n| a | b |\n|---|---|\n| c | d |\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(2),
                want_node(0),
                want_name("Title x"),
                want_file(SHOWN),
                want_span((1, 1), (2, 1)),
            ],
            &[
                want_node(11),
                want_node(2),
                want_name("A word raw end."),
                want_file(SHOWN),
                want_span((3, 1), (4, 1)),
            ],
            &[
                want_node(22),
                want_node(2),
                want_name("item one"),
                want_file(SHOWN),
                want_span((5, 1), (6, 1)),
            ],
            &[
                want_node(26),
                want_node(2),
                want_name("item two"),
                want_file(SHOWN),
                want_span((6, 1), (8, 1)),
            ],
            &[
                want_node(37),
                want_node(2),
                want_name("a"),
                want_file(SHOWN),
                want_span((8, 2), (8, 5)),
            ],
            &[
                want_node(42),
                want_node(2),
                want_name("b"),
                want_file(SHOWN),
                want_span((8, 6), (8, 9)),
            ],
            &[
                want_node(50),
                want_node(2),
                want_name("c"),
                want_file(SHOWN),
                want_span((10, 2), (10, 5)),
            ],
            &[
                want_node(55),
                want_node(2),
                want_name("d"),
                want_file(SHOWN),
                want_span((10, 6), (10, 9)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<ProseItem>();
    reading::<ProseItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
