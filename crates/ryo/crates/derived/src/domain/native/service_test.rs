//! The `native` service's cases, on values alone.

use super::service::finding;
use super::types::{NativeFinding, NativeForm};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    SpanKeyOfElements,
    NodeKey,
    FileAndStretch,
    UnsupportedShape,
}
type Input = Scenario;
type Value = Option<NativeFinding>;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_span_column_of_the_elements_is_a_span_lookup"),
        word: Word::Succeeds,
        input: || Scenario::SpanKeyOfElements,
        expect: Expect::Value(Some(NativeFinding::Span(ColumnIndex(2)))),
    },
    Case {
        name: CaseName("the_node_column_is_a_node_lookup"),
        word: Word::Succeeds,
        input: || Scenario::NodeKey,
        expect: Expect::Value(Some(NativeFinding::Node(ColumnIndex(2), ColumnIndex(0)))),
    },
    Case {
        name: CaseName("the_file_and_stretch_columns_are_a_stretch_lookup"),
        word: Word::Succeeds,
        input: || Scenario::FileAndStretch,
        expect: Expect::Value(Some(NativeFinding::Stretch(ColumnIndex(0), ColumnIndex(1)))),
    },
    Case {
        name: CaseName("an_unsupported_shape_declines_without_a_fault"),
        word: Word::Succeeds,
        input: || Scenario::UnsupportedShape,
        expect: Expect::Value(None),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let (c0, c1, c2) = (ColumnIndex(0), ColumnIndex(1), ColumnIndex(2));
    Ok(match input {
        Scenario::SpanKeyOfElements => finding(NativeForm::Elements, &[c2], &[ColumnIndex(4)], &[]),
        Scenario::NodeKey => finding(NativeForm::Edges, &[c2], &[c0], &[]),
        Scenario::FileAndStretch => finding(
            NativeForm::Elements,
            &[c0, c1],
            &[ColumnIndex(3), ColumnIndex(4)],
            &[],
        ),
        Scenario::UnsupportedShape => finding(NativeForm::Edges, &[c0, c1], &[c0, c1], &[]),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
