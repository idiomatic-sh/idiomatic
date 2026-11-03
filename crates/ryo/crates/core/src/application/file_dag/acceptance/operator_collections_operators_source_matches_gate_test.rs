//! Actual common-engine gates must precede demand-driven matcher compilation.
use super::{Held, span};
use crate::application::file_dag::testing::TestOperator as _;
use crate::domain::matching::types::prepared::value::PreparedMatchers;
use crate::domain::operating::{
    errors::OperatingError,
    types::{OperatorContext, Room},
};
use crate::domain::operator::collections::operators::source_matches::item::SourceMatchesItem;
use crate::domain::replay::types::Pipe;
use crate::domain::table_model::{
    service::push_row,
    types::{Rows, Table},
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::{
    relations::{
        constants::NO_SPAN,
        types::{CellKind, Integer, Relation, Span, Value},
    },
    symbols::types::Symbols,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, LiteralText, StepId},
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
};
fn run(source_span: Option<Span>, line: i64, stream: bool) -> (Result<(), OperatingError>, usize) {
    let operation = Operation::SourceMatches {
        from: Input::Relation(Relation::Function),
        span: ColumnIndex(0),
        base_line: Some(ColumnIndex(1)),
        pattern: LiteralText("(".into()),
        capture: None,
        capture_text: false,
        capture_line: false,
    };
    let prepared = PreparedMatchers::new(&[Plan {
        parameters: vec![],
        steps: vec![PlanStep {
            operation: operation.clone(),
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    }]);
    let mut table = Table {
        kinds: vec![CellKind::Span, CellKind::Number],
        ..Table::default()
    };
    if let Some(span) = source_span {
        push_row(
            &mut table,
            &[Value::Span(span), Value::Number(Integer(line))],
        );
    }
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: Some(&prepared),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b"x"),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let result = if stream {
        SourceMatchesItem.stream(
            &operation,
            &mut Held(&table),
            &mut context,
            &mut Room::default(),
            &mut Pipe(|_: &mut OperatorContext<'_>, _: &[Value]| Ok(())),
        )
    } else {
        SourceMatchesItem.compute(
            &operation,
            &[Rows::Computed(&table)],
            &mut context,
            &mut Table::default(),
        )
    };
    (
        result,
        prepared.builds.load(std::sync::atomic::Ordering::Relaxed),
    )
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
