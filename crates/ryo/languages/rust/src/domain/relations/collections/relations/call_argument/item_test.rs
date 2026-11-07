//! The `call_argument` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::CallArgumentItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_behind_a_reference_or_parentheses_is_passed_on_and_a_call_is_not"),
        word: Word::Succeeds,
        input: || "fn run(s: S) { stop(&mut s, make(), (s)); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(27),
                    want_number(0),
                    want_node(34),
                    want_name("s"),
                    want_file(SHOWN),
                    want_span((1, 21), (1, 27)),
                ],
                &[
                    want_node(27),
                    want_number(1),
                    want_node(45),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 29), (1, 35)),
                ],
                &[
                    want_node(27),
                    want_number(2),
                    want_node(56),
                    want_name("s"),
                    want_file(SHOWN),
                    want_span((1, 37), (1, 40)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "the_receiver_of_a_method_call_is_the_first_argument_and_a_dereference_passes_its_name",
        ),
        word: Word::Succeeds,
        input: || "fn run(s: S) { s.go(*s, 1, s.field); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(33),
                    want_number(0),
                    want_node(27),
                    want_name("s"),
                    want_file(SHOWN),
                    want_span((1, 16), (1, 17)),
                ],
                &[
                    want_node(33),
                    want_number(1),
                    want_node(37),
                    want_name("s"),
                    want_file(SHOWN),
                    want_span((1, 21), (1, 23)),
                ],
                &[
                    want_node(33),
                    want_number(2),
                    want_node(46),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 25), (1, 26)),
                ],
                &[
                    want_node(33),
                    want_number(3),
                    want_node(50),
                    want_name(""),
                    want_file(SHOWN),
                    want_span((1, 28), (1, 35)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<CallArgumentItem>();
    reading::<CallArgumentItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
