//! The `syntax_error` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxErrorItem;
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
        name: CaseName("a_rule_without_a_block_is_reported"),
        word: Word::Succeeds,
        input: || "a { b: c } d",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("unexpected end of input"),
                want_file(SHOWN),
                want_span((1, 12), (1, 13)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_broken_string_is_reported"),
        word: Word::Succeeds,
        input: || "a { b: \"c\n}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("a string a line end breaks before its closing quote"),
                want_file(SHOWN),
                want_span((1, 8), (1, 10)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_valid_stylesheet_reports_nothing"),
        word: Word::Succeeds,
        input: || "a { b: c }",
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
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
