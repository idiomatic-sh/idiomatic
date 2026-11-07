//! The `assigns` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::AssignsItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_is_bound_to_a_string_by_assignment_short_declaration_and_const"),
        word: Word::Succeeds,
        input: || "package p\nconst a = \"x\"\nfunc f() {\n\tb := \"y\"\n\tb = \"z\"\n\tc, d := \"p\", 1\n\te += \"w\"\n\t_, g := 1, \"q\"\n}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(9),
                    want_node(10),
                    want_node(15),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((2, 11), (2, 14)),
                ],
                &[
                    want_node(31),
                    want_node(32),
                    want_node(37),
                    want_node(18),
                    want_file(SHOWN),
                    want_span((4, 7), (4, 10)),
                ],
                &[
                    want_node(40),
                    want_node(41),
                    want_node(46),
                    want_node(18),
                    want_file(SHOWN),
                    want_span((5, 6), (5, 9)),
                ],
                &[
                    want_node(49),
                    want_node(50),
                    want_node(59),
                    want_node(18),
                    want_file(SHOWN),
                    want_span((6, 10), (6, 13)),
                ],
                &[
                    want_node(75),
                    want_node(80),
                    want_node(89),
                    want_node(18),
                    want_file(SHOWN),
                    want_span((8, 13), (8, 16)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "each_name_is_paired_with_the_value_at_its_place_and_a_literal_owns_its_own",
        ),
        word: Word::Succeeds,
        input: || "package p\nvar a, b = \"x\", f()\nvar g = func() {\n\th := \"in\"\n}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(9),
                    want_node(10),
                    want_node(19),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((2, 12), (2, 15)),
                ],
                &[
                    want_node(48),
                    want_node(49),
                    want_node(54),
                    want_node(38),
                    want_file(SHOWN),
                    want_span((4, 7), (4, 11)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<AssignsItem>();
    reading::<AssignsItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
