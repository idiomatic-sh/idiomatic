//! The `static_call` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StaticCallItem;
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
        name: CaseName("a_method_and_a_path_call_are_rows"),
        word: Word::Succeeds,
        input: || "fn run() { self.stop(super::make::<u8>()); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(22),
                    want_node(1),
                    want_name("stop"),
                    want_file(SHOWN),
                    want_span((1, 17), (1, 21)),
                ],
                &[
                    want_node(27),
                    want_node(1),
                    want_name("super::make"),
                    want_file(SHOWN),
                    want_span((1, 22), (1, 39)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_qualified_call_is_a_row"),
        word: Word::Succeeds,
        input: || "fn run() { crate::service(); }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(16),
                want_node(1),
                want_name("crate::service"),
                want_file(SHOWN),
                want_span((1, 12), (1, 26)),
            ]],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<StaticCallItem>();
    reading::<StaticCallItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
