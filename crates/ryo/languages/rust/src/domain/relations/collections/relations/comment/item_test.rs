//! The `comment` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::CommentItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_kind_of_comment_is_a_row"),
        word: Word::Succeeds,
        input: || "// first\n/// doc\nfn run() {} /* block */",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_name(" first"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 9)),
                    want_flag(false),
                ],
                &[
                    want_node(4),
                    want_name(" doc"),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 8)),
                    want_flag(true),
                ],
                &[
                    want_node(19),
                    want_name(" block "),
                    want_file(SHOWN),
                    want_span((3, 13), (3, 24)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("comment_rows_are_ast_comments_only"),
        word: Word::Succeeds,
        input: || "fn run() { let text = \"// not a comment\"; }",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_file_without_comments_has_no_rows"),
        word: Word::Succeeds,
        input: || "fn run() {}",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("unicode_line_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "// café",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(" café"),
                want_file(SHOWN),
                want_span((1, 1), (1, 9)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("outer_line_doc_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "/// outer",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(" outer"),
                want_file(SHOWN),
                want_span((1, 1), (1, 10)),
                want_flag(true),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("inner_line_doc_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "//! inner",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(" inner"),
                want_file(SHOWN),
                want_span((1, 1), (1, 10)),
                want_flag(true),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("four_slashes_are_plain_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "//// plain",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name("// plain"),
                want_file(SHOWN),
                want_span((1, 1), (1, 11)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("unicode_block_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "/* café */",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(" café "),
                want_file(SHOWN),
                want_span((1, 1), (1, 12)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("outer_block_doc_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "/** outer */",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(" outer "),
                want_file(SHOWN),
                want_span((1, 1), (1, 13)),
                want_flag(true),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("inner_block_doc_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "/*! inner */",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(" inner "),
                want_file(SHOWN),
                want_span((1, 1), (1, 13)),
                want_flag(true),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("three_stars_are_plain_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "/*** plain */",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name("** plain "),
                want_file(SHOWN),
                want_span((1, 1), (1, 14)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("empty_block_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "/**/",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 5)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("nested_block_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "/* outer /* nested */ tail */",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(" outer /* nested */ tail "),
                want_file(SHOWN),
                want_span((1, 1), (1, 30)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("unclosed_block_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "/* open",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(" open"),
                want_file(SHOWN),
                want_span((1, 1), (1, 8)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("empty_line_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "//",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 3)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("multiline_unicode_preserves_content_and_full_span"),
        word: Word::Succeeds,
        input: || "/* α\n β */",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(" α\n β "),
                want_file(SHOWN),
                want_span((1, 1), (2, 7)),
                want_flag(false),
            ]],
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
