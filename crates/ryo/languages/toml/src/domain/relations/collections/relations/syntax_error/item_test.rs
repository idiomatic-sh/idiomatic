//! The `syntax_error` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxErrorItem;
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
        name: CaseName("a_duplicate_key_is_reported"),
        word: Word::Succeeds,
        input: || "x = 1\nx = 2\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("duplicate key"),
                want_file(SHOWN),
                want_span((2, 1), (2, 2)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_table_defined_twice_is_reported"),
        word: Word::Succeeds,
        input: || "[a]\n[a]\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("duplicate key"),
                want_file(SHOWN),
                want_span((2, 2), (2, 3)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_inline_table_extended_is_reported"),
        word: Word::Succeeds,
        input: || "a = {b = 1}\n[a.c]\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("cannot extend value of type inline table with a dotted key"),
                want_file(SHOWN),
                want_span((2, 2), (2, 3)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_date_that_is_none_is_reported"),
        word: Word::Succeeds,
        input: || "d = 2026-99-99\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("invalid date, expected month between 01 and 12"),
                want_file(SHOWN),
                want_span((1, 5), (1, 15)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("nesting_past_the_limit_is_reported_and_read_no_deeper"),
        word: Word::Succeeds,
        input: || {
            "a = [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[1]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
"
        },
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("cannot recurse further; max recursion depth met"),
                want_file(SHOWN),
                want_span((1, 85), (1, 86)),
            ]],
            causes: &[],
        }),
    },
    Case {
        // `# c` in UTF-16LE with no byte-order mark: the bytes are UTF-8 (each NUL is a byte of text), one comment token in a tree of
        // two elements, and three reports, more than the tree has elements. A report's id is in the space of reports, never a place
        // of the table: the third report numbered as a place is past the table, and the run would refuse the whole lint.
        name: CaseName("more_reports_than_elements_are_known_by_report_ids"),
        word: Word::Succeeds,
        input: || "#\0 \0c\0",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2_147_483_648),
                    want_name("invalid comment character, expected printable characters"),
                    want_file(SHOWN),
                    want_span((1, 2), (1, 2)),
                ],
                &[
                    want_node(2_147_483_649),
                    want_name("invalid comment character, expected printable characters"),
                    want_file(SHOWN),
                    want_span((1, 4), (1, 4)),
                ],
                &[
                    want_node(2_147_483_650),
                    want_name("invalid comment character, expected printable characters"),
                    want_file(SHOWN),
                    want_span((1, 6), (1, 6)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_valid_document_has_no_error"),
        word: Word::Succeeds,
        input: || "a = 1\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<SyntaxErrorItem>();
    reading::<SyntaxErrorItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
