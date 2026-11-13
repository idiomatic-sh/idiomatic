//! The support of the `span_bounds_item` cases.
use super::super::fixtures as compiled;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, FileId, Offset, Relation, Span, Value};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId},
    plans::types::Operation,
};

pub(super) const fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}

/// What the collected run answered: its column kinds and cells, and whether the streamed run answered the same.
pub(super) struct Evaluated {
    pub(super) kinds: Vec<CellKind>,
    pub(super) cells: Vec<Value>,
    pub(super) streamed_same: bool,
}
/// The collected run's refusal, and whether the streamed run answered the same.
pub(super) struct Refusal {
    pub(super) error: OperatingError,
    pub(super) streamed_same: bool,
}
pub(super) fn evaluate(spans: &[Span]) -> Result<Evaluated, Refusal> {
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
    let streamed_same = collected == streamed;
    let output = collected.map_err(|error| Refusal {
        error,
        streamed_same,
    })?;
    Ok(Evaluated {
        kinds: output.kinds,
        cells: output.cells,
        streamed_same,
    })
}
