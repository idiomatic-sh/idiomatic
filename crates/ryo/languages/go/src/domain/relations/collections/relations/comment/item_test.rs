//! The `comment` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::CommentItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_comment_is_a_row_with_its_text_without_its_delimiters"),
        word: Word::Succeeds,
        input: || "// a\nvar x = 1 /* b\n c */\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name(" a"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 5)),
                    want_flag(true),
                ],
                &[
                    want_node(15),
                    want_name(" b\n c "),
                    want_file(SHOWN),
                    want_span((2, 11), (3, 6)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_comment_in_a_string_is_no_comment"),
        word: Word::Succeeds,
        input: || "var s = \"// no\"\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("line_and_general_comments_are_read_without_their_delimiters"),
        word: Word::Succeeds,
        input: || "package p\n\n// a\nfunc f() {} /* b */\n//\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(6),
                    want_name(" a"),
                    want_file(SHOWN),
                    want_span((3, 1), (3, 5)),
                    want_flag(true),
                ],
                &[
                    want_node(22),
                    want_name(" b "),
                    want_file(SHOWN),
                    want_span((4, 13), (4, 20)),
                    want_flag(false),
                ],
                &[
                    want_node(24),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((5, 1), (5, 3)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "a_comment_group_directly_above_a_declaration_is_documentation_and_one_a_blank_line_away_is_not",
        ),
        word: Word::Succeeds,
        input: || "// Package p.\npackage p\n\n// apart\n\nfunc f() {}\n\n// T is.\n// More.\ntype T struct {\n\t// A is.\n\tA int\n}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name(" Package p."),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 14)),
                    want_flag(true),
                ],
                &[
                    want_node(8),
                    want_name(" apart"),
                    want_file(SHOWN),
                    want_span((4, 1), (4, 9)),
                    want_flag(false),
                ],
                &[
                    want_node(24),
                    want_name(" T is."),
                    want_file(SHOWN),
                    want_span((8, 1), (8, 9)),
                    want_flag(true),
                ],
                &[
                    want_node(26),
                    want_name(" More."),
                    want_file(SHOWN),
                    want_span((9, 1), (9, 9)),
                    want_flag(true),
                ],
                &[
                    want_node(41),
                    want_name(" A is."),
                    want_file(SHOWN),
                    want_span((11, 2), (11, 10)),
                    want_flag(true),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<CommentItem>();
    reading::<CommentItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
