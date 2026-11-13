//! Cases for TypeScript `expression_shape` rows, checked against the shared reader contract: equal nodes name one node of the file
//! as their class, and a tree the parser could not read completely names none.
use super::super::fixtures::contract::check;
use super::item::ExpressionShapeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_expressions_name_one_node_as_their_class"),
        word: Word::Succeeds,
        input: || "x;\nx;\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_node(6),
                    want_file("item.ts"),
                    want_span((1, 1), (1, 3)),
                ],
                &[
                    want_node(2),
                    want_node(7),
                    want_file("item.ts"),
                    want_span((1, 1), (1, 2)),
                ],
                &[
                    want_node(6),
                    want_node(6),
                    want_file("item.ts"),
                    want_span((2, 1), (2, 3)),
                ],
                &[
                    want_node(7),
                    want_node(7),
                    want_file("item.ts"),
                    want_span((2, 1), (2, 2)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_incomplete_tree_has_no_class"),
        word: Word::Succeeds,
        input: || "x +;\n",
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
