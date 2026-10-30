//! The `string_literal` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StringLiteralItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_string_is_its_decoded_value_owned_by_its_declaration"),
        word: Word::Succeeds,
        input: || "a { content: \"x\\41\" }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(10),
                want_node(6),
                want_name("xA"),
                want_file(SHOWN),
                want_span((1, 14), (1, 20)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_unquoted_url_is_no_string"),
        word: Word::Succeeds,
        input: || "a { b: url(c.png) }",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_import_specifier_is_marked"),
        word: Word::Succeeds,
        input: || "@import \"base.css\";",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(4),
                want_node(1),
                want_name("base.css"),
                want_file(SHOWN),
                want_span((1, 9), (1, 19)),
                want_flag(true),
            ]],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<StringLiteralItem>();
    reading::<StringLiteralItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
