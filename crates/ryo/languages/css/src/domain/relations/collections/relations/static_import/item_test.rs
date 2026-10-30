//! The `static_import` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StaticImportItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span, want_unknown};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_import_is_a_row_by_the_url_it_imports"),
        word: Word::Succeeds,
        input: || "@import \"a.css\";\n@import url(b.css) screen;\n@import url(\"c.css\");",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_node(0),
                    want_unknown("imports-every-rule"),
                    want_name("a.css"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 16)),
                ],
                &[
                    want_node(7),
                    want_node(0),
                    want_unknown("imports-every-rule"),
                    want_name("b.css"),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 26)),
                ],
                &[
                    want_node(15),
                    want_node(0),
                    want_unknown("imports-every-rule"),
                    want_name("c.css"),
                    want_file(SHOWN),
                    want_span((3, 1), (3, 21)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("another_at_rule_imports_nothing"),
        word: Word::Succeeds,
        input: || "@media screen { a { b: url(c.css) } }",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<StaticImportItem>();
    reading::<StaticImportItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
