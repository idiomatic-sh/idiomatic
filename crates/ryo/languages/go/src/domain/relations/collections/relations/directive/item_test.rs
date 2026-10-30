//! The `directive` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::DirectiveItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "every_constraint_line_of_the_header_is_a_row_by_its_kind_and_its_expression",
        ),
        word: Word::Succeeds,
        input: || "//go:build linux\n// +build linux\n\npackage p\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_file(SHOWN),
                    want_span((1, 1), (1, 17)),
                    want_name("go:build"),
                    want_name("linux"),
                ],
                &[
                    want_file(SHOWN),
                    want_span((2, 1), (2, 16)),
                    want_name("+build"),
                    want_name("linux"),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_go_build_expression_is_kept_as_written"),
        word: Word::Succeeds,
        input: || "//go:build linux && amd64\n\npackage p\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_file(SHOWN),
                want_span((1, 1), (1, 26)),
                want_name("go:build"),
                want_name("linux && amd64"),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("unicode_white_space_around_the_expression_is_trimmed_as_go_trims_it"),
        word: Word::Succeeds,
        input: || "//go:build\u{a0}linux\u{85}\n\npackage p\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_file(SHOWN),
                want_span((1, 1), (1, 20)),
                want_name("go:build"),
                want_name("linux"),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_plus_build_line_without_a_space_after_the_slashes_is_a_row"),
        word: Word::Succeeds,
        input: || "//+build amd64\n\npackage p\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_file(SHOWN),
                want_span((1, 1), (1, 15)),
                want_name("+build"),
                want_name("amd64"),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_comment_that_is_no_constraint_line_is_no_row"),
        word: Word::Succeeds,
        input: || "// Package p.\n//go:buildx\n/* +build linux */\npackage p\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_go_build_line_after_the_package_clause_is_no_row"),
        word: Word::Succeeds,
        input: || "package p\n\n//go:build linux\nvar x = 1\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_constraint_after_code_on_its_line_or_below_the_header_is_no_row"),
        word: Word::Succeeds,
        input: || "package p\n\nfunc f() {} //go:build linux\n\n\t//go:build\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<DirectiveItem>();
    reading::<DirectiveItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
