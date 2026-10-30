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
                want_name("json-text"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 1)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_object_is_its_nodes_and_tokens"),
        word: Word::Succeeds,
        input: || "{\"a\":1}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("json-text"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 8)),
                ],
                &[
                    want_node(1),
                    want_name("object"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 8)),
                ],
                &[
                    want_node(2),
                    want_name("begin-object"),
                    want_name("{"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 2)),
                ],
                &[
                    want_node(3),
                    want_name("member"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 2), (1, 7)),
                ],
                &[
                    want_node(4),
                    want_name("string"),
                    want_name("\"a\""),
                    want_file(SHOWN),
                    want_span((1, 2), (1, 5)),
                ],
                &[
                    want_node(5),
                    want_name("name-separator"),
                    want_name(":"),
                    want_file(SHOWN),
                    want_span((1, 5), (1, 6)),
                ],
                &[
                    want_node(6),
                    want_name("number"),
                    want_name("1"),
                    want_file(SHOWN),
                    want_span((1, 6), (1, 7)),
                ],
                &[
                    want_node(7),
                    want_name("end-object"),
                    want_name("}"),
                    want_file(SHOWN),
                    want_span((1, 7), (1, 8)),
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
