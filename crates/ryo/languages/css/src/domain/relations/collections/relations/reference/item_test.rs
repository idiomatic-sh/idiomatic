//! The `reference` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ReferenceItem;
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
        name: CaseName("every_url_outside_an_import_is_a_reference"),
        word: Word::Succeeds,
        input: || "@import url(a.css);\na { background: url(b.png), url(\"c.png\") }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(16),
                    want_node(12),
                    want_name("b.png"),
                    want_file(SHOWN),
                    want_span((2, 17), (2, 27)),
                ],
                &[
                    want_node(19),
                    want_node(12),
                    want_name("c.png"),
                    want_file(SHOWN),
                    want_span((2, 29), (2, 41)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_namespace_url_is_no_reference"),
        word: Word::Succeeds,
        input: || "@namespace svg url(http://www.w3.org/2000/svg);",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<ReferenceItem>();
    reading::<ReferenceItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
