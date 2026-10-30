//! The `syntax_tree` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxTreeItem;
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
    name: CaseName("every_node_and_token_is_a_row"),
    word: Word::Succeeds,
    input: || "x = 1\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(0),
                want_name("module"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (2, 1)),
            ],
            &[
                want_node(1),
                want_name("assign"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 6)),
            ],
            &[
                want_node(2),
                want_name("name"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 2)),
            ],
            &[
                want_node(3),
                want_name("name-token"),
                want_name("x"),
                want_file(SHOWN),
                want_span((1, 1), (1, 2)),
            ],
            &[
                want_node(4),
                want_name("whitespace"),
                want_name(" "),
                want_file(SHOWN),
                want_span((1, 2), (1, 3)),
            ],
            &[
                want_node(5),
                want_name("equal"),
                want_name("="),
                want_file(SHOWN),
                want_span((1, 3), (1, 4)),
            ],
            &[
                want_node(6),
                want_name("whitespace"),
                want_name(" "),
                want_file(SHOWN),
                want_span((1, 4), (1, 5)),
            ],
            &[
                want_node(7),
                want_name("constant"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 5), (1, 6)),
            ],
            &[
                want_node(8),
                want_name("number"),
                want_name("1"),
                want_file(SHOWN),
                want_span((1, 5), (1, 6)),
            ],
            &[
                want_node(9),
                want_name("newline"),
                want_name("\n"),
                want_file(SHOWN),
                want_span((1, 6), (2, 1)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<SyntaxTreeItem>();
    reading::<SyntaxTreeItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
