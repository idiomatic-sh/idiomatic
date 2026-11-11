//! Physical occurrence spans and capture rows through actual compiled execution.
use super::super::fixtures as compiled;
use super::super::fixtures::span;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{CellKind, FileId, Integer, Offset, Relation, Span, Value},
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{
        ColumnIndex, Input as PlanInput, LiteralText, Operand, QueryLiteral, StepId,
    },
    plans::types::Operation,
};
fn evaluate(
    source: &'static [u8],
    spans: &[Span],
    pattern: &'static str,
) -> Result<Vec<Span>, OperatingError> {
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
        compiled::step(Operation::SourceMatches {
            from: PlanInput::Step(StepId(0)),
            span: ColumnIndex(0),
            base_line: None,
            pattern: LiteralText(pattern.into()),
            capture: None,
            capture_text: ryo_values::domain::query_operators::types::TextOutput::Omitted,
            capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
        }),
    ];
    let compiled::CompiledRun {
        result: collected, ..
    } = compiled::run(
        steps.clone(),
        &providers,
        SourceView(source),
        &mut Symbols::default(),
        compiled::Downstream::Collected,
    );
    let compiled::CompiledRun {
        result: streamed, ..
    } = compiled::run(
        steps,
        &providers,
        SourceView(source),
        &mut Symbols::default(),
        compiled::Downstream::Streamed,
    );
    assert_eq!(collected, streamed);
    let output = collected?;
    assert_eq!(output.kinds, vec![CellKind::Span, CellKind::Span]);
    Ok(output
        .cells
        .chunks_exact(2)
        .map(|row| {
            let Value::Span(span) = row[1] else {
                panic!("typed occurrence span required");
            };
            span
        })
        .collect())
}
#[test]
fn nonzero_offset_utf8_and_repeated_source_text_keep_exact_order_and_bag() {
    assert_eq!(
        evaluate("Zéé!".as_bytes(), &[span(1, 5), span(1, 5)], "é"),
        Ok(vec![span(1, 3), span(3, 5), span(1, 3), span(3, 5)])
    );
    assert_eq!(evaluate(b"abc", &[span(0, 3)], "z"), Ok(vec![]));
}

#[test]
fn zero_width_and_absent_spans_are_explicit() {
    assert_eq!(
        evaluate("éx".as_bytes(), &[span(0, 3)], ""),
        Ok(vec![span(0, 0), span(2, 2), span(3, 3)])
    );
    assert_eq!(
        evaluate(b"abc", &[span(1, 1), NO_SPAN], ""),
        Ok(vec![span(1, 1)])
    );
}

#[test]
fn malformed_or_non_utf8_boundary_spans_refuse_before_their_row_emits() {
    for (source, invalid) in [
        (&b"abc"[..], span(2, 1)),
        (&b"abc"[..], span(0, 4)),
        ("Zé!".as_bytes(), span(2, 3)),
    ] {
        assert_eq!(
            evaluate(source, &[invalid], "é"),
            Err(OperatingError::InvalidSourceMatchSpan)
        );
    }
    assert_eq!(
        evaluate(b"a\xffb", &[span(0, 3)], "a"),
        Err(OperatingError::InvalidSourceMatchSpan),
        "a whole-file match must never invent offsets in lossy-decoded source",
    );
}

fn evaluate_lines(
    source: &'static [u8],
    source_span: Span,
    base_line: i64,
    pattern: &'static str,
) -> Result<Vec<(Span, i64)>, OperatingError> {
    let providers = [compiled::Provider {
        relation: Relation::SyntaxTree,
        table: compiled::rows(&[compiled::FileSpan {
            file: FileId(0),
            span: source_span,
        }]),
    }];
    let steps = vec![
        compiled::select(Relation::SyntaxTree, &[ColumnIndex(4)]),
        compiled::step(Operation::Extend {
            from: PlanInput::Step(StepId(0)),
            values: vec![Operand::Literal(QueryLiteral::Number(Integer(base_line)))],
        }),
        compiled::step(Operation::SourceMatches {
            from: PlanInput::Step(StepId(1)),
            span: ColumnIndex(0),
            base_line: Some(ColumnIndex(1)),
            pattern: LiteralText(pattern.into()),
            capture: None,
            capture_text: ryo_values::domain::query_operators::types::TextOutput::Omitted,
            capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
        }),
    ];
    let compiled::CompiledRun { result: result, .. } = compiled::run(
        steps,
        &providers,
        SourceView(source),
        &mut Symbols::default(),
        compiled::Downstream::Collected,
    );
    Ok(result?
        .cells
        .chunks_exact(4)
        .map(|row| match (row[2], row[3]) {
            (Value::Span(span), Value::Number(line)) => (span, line.0),
            _ => panic!("span and physical start line required"),
        })
        .collect())
}
#[test]
fn optional_line_uses_match_start_and_counts_crlf_and_single_endings_once() {
    assert_eq!(
        evaluate_lines(b"a\r\nb\rc\nd", span(0, 8), 1, "\nb|b\\rc|\nd|c"),
        Ok(vec![(span(2, 4), 1), (span(5, 6), 3), (span(6, 8), 3)])
    );
    assert_eq!(
        evaluate_lines(b"a\r\nb", span(0, 4), 1, ""),
        Ok(vec![
            (span(0, 0), 1),
            (span(1, 1), 1),
            (span(2, 2), 1),
            (span(3, 3), 2),
            (span(4, 4), 2)
        ])
    );
}

#[test]
fn optional_line_rejects_zero_and_overflow() {
    assert_eq!(
        evaluate_lines(b"x", span(0, 1), 0, "x"),
        Err(OperatingError::InvalidSourceMatchSpan)
    );
    assert_eq!(
        evaluate_lines(b"a\nb", span(0, 3), i64::MAX, "b"),
        Err(OperatingError::InvalidSourceMatchSpan)
    );
}

fn captured(
    source: &'static [u8],
    spans: &[Span],
    pattern: &'static str,
) -> Result<Vec<(Span, Span, String)>, OperatingError> {
    use ryo_language::domain::symbols::{
        service::{intern_derived, text_of},
        types::ByteSlice,
    };
    use ryo_relations::domain::terms::types::NameText;
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
        compiled::step(Operation::SourceMatches {
            from: PlanInput::Step(StepId(0)),
            span: ColumnIndex(0),
            base_line: None,
            pattern: LiteralText(pattern.into()),
            capture: Some(LiteralText("piece".into())),
            capture_text: ryo_values::domain::query_operators::types::TextOutput::Appended,
            capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
        }),
    ];
    let mut symbols = Symbols::default();
    let comment = intern_derived(&mut symbols, ByteSlice(source), NameText(" noqa: W605"));
    let compiled::CompiledRun { result: result, .. } = compiled::run(
        steps,
        &providers,
        SourceView(source),
        &mut symbols,
        compiled::Downstream::Collected,
    );
    let output = result?;
    assert_eq!(
        text_of(&symbols, ByteSlice(source), comment).map(|text| text.0),
        Some(" noqa: W605")
    );
    assert_eq!(
        output.kinds,
        vec![
            CellKind::Span,
            CellKind::Span,
            CellKind::Span,
            CellKind::Symbol
        ]
    );
    Ok(output
        .cells
        .chunks_exact(4)
        .map(|row| {
            let (Value::Span(whole), Value::Span(piece), Value::Symbol(symbol)) =
                (row[1], row[2], row[3])
            else {
                panic!("typed capture outputs required")
            };
            (
                whole,
                piece,
                text_of(&symbols, ByteSlice(source), symbol)
                    .expect("captured symbol")
                    .0
                    .to_string(),
            )
        })
        .collect())
}
#[test]
fn named_capture_preserves_offsets_order_bag_and_skips_unmatched_branches() {
    assert_eq!(
        captured(
            "é\\q\\z".as_bytes(),
            &[span(2, 6), span(2, 6)],
            r"(?P<piece>\\.)"
        ),
        Ok(vec![
            (span(2, 4), span(2, 4), "\\q".into()),
            (span(4, 6), span(4, 6), "\\z".into()),
            (span(2, 4), span(2, 4), "\\q".into()),
            (span(4, 6), span(4, 6), "\\z".into()),
        ])
    );
    assert_eq!(
        captured(b"\\qz", &[span(0, 3)], r"(?P<piece>\\q)|z"),
        Ok(vec![(span(0, 2), span(0, 2), "\\q".into())])
    );
    assert_eq!(
        captured("é".as_bytes(), &[span(0, 2)], r"(?P<piece>)"),
        Ok(vec![
            (span(0, 0), span(0, 0), "".into()),
            (span(2, 2), span(2, 2), "".into())
        ])
    );
}
