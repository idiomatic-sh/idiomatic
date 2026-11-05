//! The `syntax_error` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxErrorItem;
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
        name: CaseName("syntax_error_is_reported_not_fatal"),
        word: Word::Succeeds,
        input: || "fn",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2_147_483_648),
                    want_name("expected a name"),
                    want_name("parse"),
                    want_name("incomplete"),
                    want_name("unspecified"),
                    want_file(SHOWN),
                    want_span((1, 3), (1, 3)),
                ],
                &[
                    want_node(2_147_483_649),
                    want_name("expected function arguments"),
                    want_name("parse"),
                    want_name("incomplete"),
                    want_name("unspecified"),
                    want_file(SHOWN),
                    want_span((1, 3), (1, 3)),
                ],
                &[
                    want_node(2_147_483_650),
                    want_name("expected a block"),
                    want_name("parse"),
                    want_name("incomplete"),
                    want_name("unspecified"),
                    want_file(SHOWN),
                    want_span((1, 3), (1, 3)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_whole_item_has_none"),
        word: Word::Succeeds,
        input: || "fn run() {}",
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
