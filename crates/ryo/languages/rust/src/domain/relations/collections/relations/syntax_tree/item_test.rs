//! The `syntax_tree` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxTreeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_file_is_one_node"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(0),
                want_name("source-file"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 1)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("every_node_and_token_is_a_row"),
        word: Word::Succeeds,
        input: || "fn run() {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("source-file"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 12)),
                ],
                &[
                    want_node(1),
                    want_name("fn"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 12)),
                ],
                &[
                    want_node(2),
                    want_name("fn-kw"),
                    want_name("fn"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 3)),
                ],
                &[
                    want_node(3),
                    want_name("whitespace"),
                    want_name(" "),
                    want_file(SHOWN),
                    want_span((1, 3), (1, 4)),
                ],
                &[
                    want_node(4),
                    want_name("name"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 7)),
                ],
                &[
                    want_node(5),
                    want_name("ident"),
                    want_name("run"),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 7)),
                ],
                &[
                    want_node(6),
                    want_name("param-list"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 7), (1, 9)),
                ],
                &[
                    want_node(7),
                    want_name("l-paren"),
                    want_name("("),
                    want_file(SHOWN),
                    want_span((1, 7), (1, 8)),
                ],
                &[
                    want_node(8),
                    want_name("r-paren"),
                    want_name(")"),
                    want_file(SHOWN),
                    want_span((1, 8), (1, 9)),
                ],
                &[
                    want_node(9),
                    want_name("whitespace"),
                    want_name(" "),
                    want_file(SHOWN),
                    want_span((1, 9), (1, 10)),
                ],
                &[
                    want_node(10),
                    want_name("block-expr"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 12)),
                ],
                &[
                    want_node(11),
                    want_name("stmt-list"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 12)),
                ],
                &[
                    want_node(12),
                    want_name("l-curly"),
                    want_name("{"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 11)),
                ],
                &[
                    want_node(13),
                    want_name("r-curly"),
                    want_name("}"),
                    want_file(SHOWN),
                    want_span((1, 11), (1, 12)),
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
