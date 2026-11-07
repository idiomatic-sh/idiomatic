//! Identical classes retain separate source occurrences; invalid trees never share a fake unknown class.
use super::super::fixtures::contract::check;
use super::item::ExpressionShapeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal expressions have one opaque class and two occurrences"),
        word: Word::Succeeds,
        input: || "x\nx\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 2)),
                ],
                &[
                    want_node(6),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 2)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("unrelated recovered expressions do not share an unknown class"),
        word: Word::Succeeds,
        input: || "x +\ny *\n",
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
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
    run(CHECKS, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}

fn module_nonlocal_validation_keeps_expression_shapes_complete() {
    let result = subject("nonlocal x\n1\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 1);
}

/// Each check below states its own source and asserts its own outcome.
const CHECKS: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("module_nonlocal_validation_keeps_expression_shapes_complete"),
        word: Word::Succeeds,
        input: || module_nonlocal_validation_keeps_expression_shapes_complete,
        expect: Expect::Value(()),
    },
];
