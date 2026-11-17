//! The `expression_shape` item's cases, and the collection's shared cases: equal nodes name one node of the file as their class,
//! and a tree the parser could not read completely names none.

use super::super::fixtures::contract::check;
use super::item::ExpressionShapeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_expressions_name_one_node_as_their_class"),
        word: Word::Succeeds,
        input: || "type A = x;\ntype B = x;\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_node(1),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 12)),
                ],
                &[
                    want_node(4),
                    want_node(4),
                    want_file(SHOWN),
                    want_span((1, 6), (1, 7)),
                ],
                &[
                    want_node(9),
                    want_node(24),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 11)),
                ],
                &[
                    want_node(10),
                    want_node(25),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 11)),
                ],
                &[
                    want_node(11),
                    want_node(26),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 11)),
                ],
                &[
                    want_node(12),
                    want_node(27),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 11)),
                ],
                &[
                    want_node(16),
                    want_node(16),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 12)),
                ],
                &[
                    want_node(19),
                    want_node(19),
                    want_file(SHOWN),
                    want_span((2, 6), (2, 7)),
                ],
                &[
                    want_node(24),
                    want_node(24),
                    want_file(SHOWN),
                    want_span((2, 10), (2, 11)),
                ],
                &[
                    want_node(25),
                    want_node(25),
                    want_file(SHOWN),
                    want_span((2, 10), (2, 11)),
                ],
                &[
                    want_node(26),
                    want_node(26),
                    want_file(SHOWN),
                    want_span((2, 10), (2, 11)),
                ],
                &[
                    want_node(27),
                    want_node(27),
                    want_file(SHOWN),
                    want_span((2, 10), (2, 11)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_incomplete_tree_has_no_class"),
        word: Word::Succeeds,
        input: || "type A = ;\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["expression shapes require a complete syntax tree"],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<ExpressionShapeItem>();
    reading::<ExpressionShapeItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
