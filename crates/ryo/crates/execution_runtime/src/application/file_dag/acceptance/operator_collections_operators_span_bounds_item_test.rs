//! Endpoint projection through actual compiled owner execution.
use super::fixtures as compiled;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{CellKind, FileId, Integer, Offset, Relation, Span, Value},
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId},
    plans::types::Operation,
};
fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}
fn evaluate(spans: &[Span]) -> Result<Vec<Value>, OperatingError> {
    let providers = [compiled::Provider {
        relation: Relation::SyntaxTree,
        table: compiled::rows(
            &spans
                .iter()
                .copied()
                .map(|span| compiled::FileSpan {
                    file: FileId(0),
                    span,
                })
                .collect::<Vec<_>>(),
        ),
    }];
    let steps = vec![
        compiled::select(Relation::SyntaxTree, &[ColumnIndex(4)]),
        compiled::step(Operation::SpanBounds {
            from: PlanInput::Step(StepId(0)),
            span: ColumnIndex(0),
        }),
    ];
    let compiled::CompiledRun {
        result: collected, ..
    } = compiled::run(
        steps.clone(),
        &providers,
        SourceView(b"abcdef"),
        &mut Symbols::default(),
        compiled::Downstream::Collected,
    );
    let compiled::CompiledRun {
        result: streamed, ..
    } = compiled::run(
        steps,
        &providers,
        SourceView(b"abcdef"),
        &mut Symbols::default(),
        compiled::Downstream::Streamed,
    );
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
    let providers = [compiled::Provider {
        relation: Relation::SyntaxTree,
        table: compiled::rows(&[
            compiled::FileSpan {
                file: FileId(1),
                span: span(2, 4),
            },
            compiled::FileSpan {
                file: FileId(2),
                span: span(2, 4),
            },
        ]),
    }];
    let steps = vec![
        compiled::select(Relation::SyntaxTree, &[ColumnIndex(3), ColumnIndex(4)]),
        compiled::step(Operation::SpanBounds {
            from: PlanInput::Step(StepId(0)),
            span: ColumnIndex(1),
        }),
    ];
    let compiled::CompiledRun { result: result, .. } = compiled::run(
        steps,
        &providers,
        SourceView(b""),
        &mut Symbols::default(),
        compiled::Downstream::Collected,
    );
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
