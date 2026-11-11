//! The support of the `source_matches_item` cases.
use super::super::fixtures as compiled;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Relation, Span, Value};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{
        ColumnIndex, Input as PlanInput, LiteralText, Operand, QueryLiteral, StepId,
    },
    plans::types::Operation,
};

pub(super) fn evaluate(
    source: &'static [u8],
    spans: &[Span],
    pattern: &'static str,
) -> Result<(Vec<Span>, Vec<CellKind>, bool), OperatingError> {
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
    let streamed_same = collected == streamed;
    let output = collected?;
    let spans = output
        .cells
        .chunks_exact(2)
        .map(|row| {
            let Value::Span(span) = row[1] else {
                panic!("typed occurrence span required");
            };
            span
        })
        .collect();
    Ok((spans, output.kinds, streamed_same))
}

pub(super) fn evaluate_lines(
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

pub(super) fn captured(
    source: &'static [u8],
    spans: &[Span],
    pattern: &'static str,
) -> Result<(Vec<(Span, Span, String)>, Vec<CellKind>, Option<String>), OperatingError> {
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
    let comment = text_of(&symbols, ByteSlice(source), comment).map(|text| text.0.to_owned());
    let captures = output
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
        .collect();
    Ok((captures, output.kinds, comment))
}
