//! Real compiled source/line gates precede deferred matcher compilation.
use super::operator_collections_operators_source_matches_item_test::span;
use crate::application::file_dag::acceptance::compiled_span_fixture as compiled;
use crate::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{FileId, Integer, Relation, Span},
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{ColumnIndex, Input, LiteralText, Operand, QueryLiteral, StepId},
    plans::types::Operation,
};
fn run(source_span: Option<Span>, line: i64, stream: bool) -> (Result<(), OperatingError>, usize) {
    let spans = source_span
        .into_iter()
        .map(|span| compiled::FileSpan {
            file: FileId(0),
            span,
        })
        .collect::<Vec<_>>();
    let providers = [compiled::Provider {
        relation: Relation::SyntaxTree,
        table: compiled::rows(&spans),
    }];
    let downstream = if stream {
        compiled::Downstream::Streamed
    } else {
        compiled::Downstream::Collected
    };
    let steps = vec![
        compiled::select(Relation::SyntaxTree, &[ColumnIndex(4)]),
        compiled::step(Operation::Extend {
            from: Input::Step(StepId(0)),
            values: vec![Operand::Literal(QueryLiteral::Number(Integer(line)))],
        }),
        compiled::step(Operation::SourceMatches {
            from: Input::Step(StepId(1)),
            span: ColumnIndex(0),
            base_line: Some(ColumnIndex(1)),
            pattern: LiteralText("(".into()),
            capture: None,
            capture_text: ryo_values::domain::query_operators::types::TextOutput::Omitted,
            capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
        }),
    ];
    let compiled::CompiledRun { result, builds } = compiled::run(
        steps,
        &providers,
        SourceView(b"x"),
        &mut Symbols::default(),
        downstream,
    );
    (result.map(|_| ()), builds.0)
}
#[test]
fn malformed_source_and_line_refuse_without_acquiring_bad_regex() {
    for stream in [false, true] {
        assert_eq!(
            run(Some(span(0, 2)), 1, stream),
            (Err(OperatingError::InvalidSourceMatchSpan), 0)
        );
        assert_eq!(
            run(Some(span(0, 1)), 0, stream),
            (Err(OperatingError::InvalidSourceMatchSpan), 0)
        );
        assert_eq!(run(Some(NO_SPAN), 0, stream), (Ok(()), 0));
        assert_eq!(run(None, 1, stream), (Ok(()), 0));
        let (result, builds) = run(Some(span(0, 1)), 1, stream);
        assert!(matches!(result, Err(OperatingError::InvalidRegex { .. })));
        assert_eq!(builds, 1);
    }
}
