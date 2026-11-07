//! The `visibility` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::VisibilityItem;
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
        name: CaseName("a_visibility_is_written_on_its_item"),
        word: Word::Succeeds,
        input: || "pub(crate) fn run() {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_node(1),
                want_name("pub(crate)"),
                want_file(SHOWN),
                want_span((1, 1), (1, 11)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("white_space_in_a_visibility_is_one_space"),
        word: Word::Succeeds,
        input: || "pub ( in  crate::a ) struct S;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_node(1),
                want_name("pub ( in crate::a )"),
                want_file(SHOWN),
                want_span((1, 1), (1, 21)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_public_use_is_written_on_each_name_it_binds"),
        word: Word::Succeeds,
        input: || "pub use a::{b, c};",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_node(15),
                    want_name("pub"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 4)),
                ],
                &[
                    want_node(2),
                    want_node(22),
                    want_name("pub"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 4)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_item_written_without_one_has_no_row"),
        word: Word::Succeeds,
        input: || "fn run() {}\nimpl S { fn go() {} }",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<VisibilityItem>();
    reading::<VisibilityItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
