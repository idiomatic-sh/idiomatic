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
                want_name("file"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 1)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_tokens_of_a_file_spell_it_with_its_white_space"),
        word: Word::Succeeds,
        input: || "package p\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("file"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (2, 1)),
                ],
                &[
                    want_node(1),
                    want_name("package-token"),
                    want_name("package"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 8)),
                ],
                &[
                    want_node(2),
                    want_name("whitespace"),
                    want_name(" "),
                    want_file(SHOWN),
                    want_span((1, 8), (1, 9)),
                ],
                &[
                    want_node(3),
                    want_name("ident"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 9), (1, 10)),
                ],
                &[
                    want_node(4),
                    want_name("ident-token"),
                    want_name("p"),
                    want_file(SHOWN),
                    want_span((1, 9), (1, 10)),
                ],
                &[
                    want_node(5),
                    want_name("whitespace"),
                    want_name("\n"),
                    want_file(SHOWN),
                    want_span((1, 10), (2, 1)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("every_node_and_token_is_a_row_with_the_text_of_its_token"),
        word: Word::Succeeds,
        input: || "package p\nvar a = 1 // c\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("file"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 1), (3, 1)),
                ],
                &[
                    want_node(1),
                    want_name("package-token"),
                    want_name("package"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 8)),
                ],
                &[
                    want_node(2),
                    want_name("whitespace"),
                    want_name(" "),
                    want_file(SHOWN),
                    want_span((1, 8), (1, 9)),
                ],
                &[
                    want_node(3),
                    want_name("ident"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 9), (1, 10)),
                ],
                &[
                    want_node(4),
                    want_name("ident-token"),
                    want_name("p"),
                    want_file(SHOWN),
                    want_span((1, 9), (1, 10)),
                ],
                &[
                    want_node(5),
                    want_name("whitespace"),
                    want_name("\n"),
                    want_file(SHOWN),
                    want_span((1, 10), (2, 1)),
                ],
                &[
                    want_node(6),
                    want_name("gen-decl"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 10)),
                ],
                &[
                    want_node(7),
                    want_name("var"),
                    want_name("var"),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 4)),
                ],
                &[
                    want_node(8),
                    want_name("whitespace"),
                    want_name(" "),
                    want_file(SHOWN),
                    want_span((2, 4), (2, 5)),
                ],
                &[
                    want_node(9),
                    want_name("value-spec"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((2, 5), (2, 10)),
                ],
                &[
                    want_node(10),
                    want_name("ident"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((2, 5), (2, 6)),
                ],
                &[
                    want_node(11),
                    want_name("ident-token"),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((2, 5), (2, 6)),
                ],
                &[
                    want_node(12),
                    want_name("whitespace"),
                    want_name(" "),
                    want_file(SHOWN),
                    want_span((2, 6), (2, 7)),
                ],
                &[
                    want_node(13),
                    want_name("assign"),
                    want_name("="),
                    want_file(SHOWN),
                    want_span((2, 7), (2, 8)),
                ],
                &[
                    want_node(14),
                    want_name("whitespace"),
                    want_name(" "),
                    want_file(SHOWN),
                    want_span((2, 8), (2, 9)),
                ],
                &[
                    want_node(15),
                    want_name("basic-lit"),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((2, 9), (2, 10)),
                ],
                &[
                    want_node(16),
                    want_name("int"),
                    want_name("1"),
                    want_file(SHOWN),
                    want_span((2, 9), (2, 10)),
                ],
                &[
                    want_node(17),
                    want_name("whitespace"),
                    want_name(" "),
                    want_file(SHOWN),
                    want_span((2, 10), (2, 11)),
                ],
                &[
                    want_node(18),
                    want_name("comment-token"),
                    want_name("// c"),
                    want_file(SHOWN),
                    want_span((2, 11), (2, 15)),
                ],
                &[
                    want_node(19),
                    want_name("whitespace"),
                    want_name("\n"),
                    want_file(SHOWN),
                    want_span((2, 15), (3, 1)),
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
