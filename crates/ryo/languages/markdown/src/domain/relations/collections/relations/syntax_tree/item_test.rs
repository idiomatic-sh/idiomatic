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
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_node_and_token_is_a_row_the_markers_included"),
        word: Word::Succeeds,
        input: || "# A\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("root"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (2, 1)),
                ],
                &[
                    want_node(1),
                    want_name("section"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (2, 1)),
                ],
                &[
                    want_node(2),
                    want_name("heading"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (2, 1)),
                ],
                &[
                    want_node(3),
                    want_name("markup"),
                    want_name("# "),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 3)),
                ],
                &[
                    want_node(4),
                    want_name("text"),
                    want_name("A"),
                    want_file(SHOWN),
                    want_span((1, 3), (1, 4)),
                ],
                &[
                    want_node(5),
                    want_name("whitespace"),
                    want_name("\n"),
                    want_file(SHOWN),
                    want_span((1, 4), (2, 1)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_span_is_a_node_around_its_text"),
        word: Word::Succeeds,
        input: || "*a* b\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("root"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (2, 1)),
                ],
                &[
                    want_node(1),
                    want_name("paragraph"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (2, 1)),
                ],
                &[
                    want_node(2),
                    want_name("emphasis"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 4)),
                ],
                &[
                    want_node(3),
                    want_name("markup"),
                    want_name("*"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 2)),
                ],
                &[
                    want_node(4),
                    want_name("text"),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 2), (1, 3)),
                ],
                &[
                    want_node(5),
                    want_name("markup"),
                    want_name("*"),
                    want_file(SHOWN),
                    want_span((1, 3), (1, 4)),
                ],
                &[
                    want_node(6),
                    want_name("text"),
                    want_name(" b"),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 6)),
                ],
                &[
                    want_node(7),
                    want_name("whitespace"),
                    want_name("\n"),
                    want_file(SHOWN),
                    want_span((1, 6), (2, 1)),
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
