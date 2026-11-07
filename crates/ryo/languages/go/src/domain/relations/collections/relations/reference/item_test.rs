//! The `reference` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ReferenceItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span, want_unknown};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_dotted_name_is_one_reference_and_a_declared_name_or_label_is_none"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\ta.b.c = d\n\te := g()\n\th(e)\nL:\n\tgoto L\n}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(20),
                    want_node(6),
                    want_name("a.b.c"),
                    want_file(SHOWN),
                    want_span((3, 2), (3, 7)),
                ],
                &[
                    want_node(33),
                    want_node(6),
                    want_name("d"),
                    want_file(SHOWN),
                    want_span((3, 10), (3, 11)),
                ],
                &[
                    want_node(43),
                    want_node(6),
                    want_name("g"),
                    want_file(SHOWN),
                    want_span((4, 7), (4, 8)),
                ],
                &[
                    want_node(50),
                    want_node(6),
                    want_name("h"),
                    want_file(SHOWN),
                    want_span((5, 2), (5, 3)),
                ],
                &[
                    want_node(53),
                    want_node(6),
                    want_name("e"),
                    want_file(SHOWN),
                    want_span((5, 4), (5, 5)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_key_of_a_composite_literal_has_the_unknown_value_of_an_ambiguous_key"),
        word: Word::Succeeds,
        input: || "package p\nvar a = T{k: v, w}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(16),
                    want_node(0),
                    want_name("T"),
                    want_file(SHOWN),
                    want_span((2, 9), (2, 10)),
                ],
                &[
                    want_node(20),
                    want_node(0),
                    want_unknown("ambiguous-key"),
                    want_file(SHOWN),
                    want_span((2, 11), (2, 12)),
                ],
                &[
                    want_node(24),
                    want_node(0),
                    want_name("v"),
                    want_file(SHOWN),
                    want_span((2, 14), (2, 15)),
                ],
                &[
                    want_node(28),
                    want_node(0),
                    want_name("w"),
                    want_file(SHOWN),
                    want_span((2, 17), (2, 18)),
                ],
            ],
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
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
