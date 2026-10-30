//! The `syntax_tree` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxTreeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_file_is_its_root_node_alone"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(0),
                want_name("program"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 1)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_text_of_a_file_is_a_token_under_its_root"),
        word: Word::Succeeds,
        input: || "a",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("program"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 2)),
                ],
                &[
                    want_node(1),
                    want_name("expression-statement"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 2)),
                ],
                &[
                    want_node(2),
                    want_name("identifier"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 2)),
                ],
                &[
                    want_node(3),
                    want_name("identifier-token"),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 2)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<SyntaxTreeItem>();
    reading::<SyntaxTreeItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
