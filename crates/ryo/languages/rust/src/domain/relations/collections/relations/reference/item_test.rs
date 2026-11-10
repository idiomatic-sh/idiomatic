//! The `reference` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ReferenceItem;
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
        name: CaseName("unresolved_reference_keeps_its_name_and_scope_id"),
        word: Word::Succeeds,
        input: || "fn run() { missing(); }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(17),
                want_node(1),
                want_name("missing"),
                want_file(SHOWN),
                want_span((1, 12), (1, 19)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("every_outermost_path_is_a_reference"),
        word: Word::Succeeds,
        input: || "fn run(value: crate::Value) { consume(value); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(15),
                    want_node(1),
                    want_name("crate::Value"),
                    want_file(SHOWN),
                    want_span((1, 15), (1, 27)),
                ],
                &[
                    want_node(33),
                    want_node(1),
                    want_name("consume"),
                    want_file(SHOWN),
                    want_span((1, 31), (1, 38)),
                ],
                &[
                    want_node(40),
                    want_node(1),
                    want_name("value"),
                    want_file(SHOWN),
                    want_span((1, 39), (1, 44)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_path_of_a_use_is_an_import_not_a_reference"),
        word: Word::Succeeds,
        input: || "use crate::service::Handle;\nstatic HANDLE: Handle = Handle::new();",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(29),
                    want_node(0),
                    want_name("Handle"),
                    want_file(SHOWN),
                    want_span((2, 16), (2, 22)),
                ],
                &[
                    want_node(38),
                    want_node(0),
                    want_name("Handle::new"),
                    want_file(SHOWN),
                    want_span((2, 25), (2, 36)),
                ],
            ],
            causes: &[],
        }),
    },
];

pub(super) fn subject(source: Input) -> Value {
    check::<ReferenceItem>();
    reading::<ReferenceItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
