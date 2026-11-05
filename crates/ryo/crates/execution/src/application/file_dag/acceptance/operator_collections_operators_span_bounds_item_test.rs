//! Endpoint projection through actual compiled owner execution.
use super::compiled_span_fixture as compiled;
use crate::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{CellKind, FileId, Integer, Offset, Relation, Span, Value},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId},
    plans::types::{Operation, QueryOperator},
};
fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}
fn evaluate(spans: &[Span]) -> Result<Vec<Value>, OperatingError> {
    let providers = [(
        Relation::SyntaxTree,
        compiled::rows(
            &spans
                .iter()
                .copied()
                .map(|span| (FileId(0), span))
                .collect::<Vec<_>>(),
        ),
    )];
    let steps = vec![
        compiled::select(Relation::SyntaxTree, &[4]),
        compiled::step(Operation::SpanBounds {
            from: PlanInput::Step(StepId(0)),
            span: ColumnIndex(0),
        }),
    ];
    let (collected, _) = compiled::run(
        steps.clone(),
        &providers,
        b"abcdef",
        &mut Symbols::default(),
        false,
    );
    let (streamed, _) = compiled::run(steps, &providers, b"abcdef", &mut Symbols::default(), true);
    assert_eq!(collected, streamed);
    let output = collected?;
    assert_eq!(
        output.kinds,
        vec![CellKind::Span, CellKind::Number, CellKind::Number]
    );
    Ok(output.cells)
}
#[test]
fn checked_bounds_preserve_duplicate_rows_and_order() {
    assert_eq!(
        evaluate(&[span(2, 4), span(2, 4), span(0, 0)]),
        Ok(vec![
            Value::Span(span(2, 4)),
            Value::Number(Integer(2)),
            Value::Number(Integer(4)),
            Value::Span(span(2, 4)),
            Value::Number(Integer(2)),
            Value::Number(Integer(4)),
            Value::Span(span(0, 0)),
            Value::Number(Integer(0)),
            Value::Number(Integer(0)),
        ])
    );
}

#[test]
fn absent_and_reversed_spans_refuse_before_row_output() {
    for invalid in [NO_SPAN, span(4, 3)] {
        assert_eq!(evaluate(&[invalid]), Err(OperatingError::InvalidSpanBounds));
    }
    assert_eq!(
        evaluate(&[span(0, 7)]),
        Ok(vec![
            Value::Span(span(0, 7)),
            Value::Number(Integer(0)),
            Value::Number(Integer(7)),
        ]),
        "pure endpoint extraction does not claim the source bytes of a cross-file row"
    );
}

#[test]
fn equal_offsets_from_distinct_files_keep_both_file_identities() {
    let providers = [(
        Relation::SyntaxTree,
        compiled::rows(&[(FileId(1), span(2, 4)), (FileId(2), span(2, 4))]),
    )];
    let steps = vec![
        compiled::select(Relation::SyntaxTree, &[3, 4]),
        compiled::step(Operation::SpanBounds {
            from: PlanInput::Step(StepId(0)),
            span: ColumnIndex(1),
        }),
    ];
    let (result, _) = compiled::run(steps, &providers, b"", &mut Symbols::default(), false);
    assert_eq!(
        result.expect("pure cross-file projection").cells,
        vec![
            Value::File(FileId(1)),
            Value::Span(span(2, 4)),
            Value::Number(Integer(2)),
            Value::Number(Integer(4)),
            Value::File(FileId(2)),
            Value::Span(span(2, 4)),
            Value::Number(Integer(2)),
            Value::Number(Integer(4))
        ]
    );
}
#[test]
fn contract() {
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::SpanBounds,
    );
}
