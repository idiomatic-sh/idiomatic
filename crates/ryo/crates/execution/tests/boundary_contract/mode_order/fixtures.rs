//! The support of the `mode_order` cases and of the groups nested in it.
use super::super::*;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::Direction,
    row_texts::types::Cell as HostCell,
    rows::errors::RowsError,
};

/// The root rule 0 of `input` delivered in file 0 run through `run_file` with its diagnostic collected over `facts`, or what
/// stopped it.
pub(super) fn evaluate(
    input: PrepareRequest,
    facts: &mut TableFacts,
) -> Result<RootOutcome, FileStop> {
    let engine = engine();
    let prepared = engine
        .prepare(input)
        .expect("prepare actual owner policy case");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut rules = run_file(
        &request,
        FileRun::of(
            &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }],
            &elements,
        ),
        &mut symbols,
        facts,
    );
    let outcome = root(ran(&mut rules, 0));
    request.finish();
    outcome
}

pub(super) fn step(operation: Operation) -> PlanStep {
    PlanStep {
        written: Default::default(),
        scope: StepScope::PerFile,
        operation,
    }
}

pub(super) fn select(relation: Relation) -> Operation {
    Operation::Select {
        from: Input::Relation(relation),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    }
}

pub(super) fn hosted(values: &[i64]) -> HostRelation {
    HostRelation {
        name: HostName("numbers".into()),
        columns: vec![HostColumn {
            name: HostColumnName("number".into()),
            kind: HostKind::Number,
        }],
        rows: values
            .iter()
            .map(|value| HostRow(vec![HostCell::Number(Integer(*value))]))
            .collect(),
    }
}

pub(super) fn axis(from: Input, domain: Input, parent: u32) -> Operation {
    Operation::Axis {
        from,
        edges: Input::Relation(Relation::SemanticChild),
        domain,
        direction: Direction::Child,
        id: ColumnIndex(0),
        parent: ColumnIndex(parent),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(99),
        domain_filter: vec![],
        domain_view: vec![],
        output: vec![],
    }
}

/// The column a root failed for want of; `None` when it did not fail on a missing column.
pub(super) fn failed_column(outcome: Result<RootOutcome, FileStop>) -> Option<u32> {
    if let Ok(RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn { index }))) =
        outcome
    {
        Some(index)
    } else {
        None
    }
}

/// How a step's answer shows in the row-by-row cases.
pub(super) fn described(outcome: Result<RootOutcome, FileStop>) -> String {
    match outcome {
        Ok(RootOutcome::Complete(table)) => format!("complete {:?} {:?}", table.kinds, table.count),
        Ok(RootOutcome::Partial { table, .. }) => format!("partial {:?}", table.kinds),
        Ok(RootOutcome::Skipped(_)) => "skipped".into(),
        Ok(RootOutcome::Unknown(error)) => format!("unknown {error:?}"),
        Ok(RootOutcome::Failed(error)) => format!("failed {error:?}"),
        Ok(RootOutcome::NoQuery) => "noquery".into(),
        Ok(RootOutcome::Deferred { .. }) => "deferred".into(),
        Err(_) => "filestop".into(),
    }
}

/// Each operator that reads its operands row by row, over a column past its input or of the wrong kind (or a sound one, as a
/// control), by name.
pub(super) fn row_by_row_steps() -> Vec<(&'static str, Operation)> {
    use ryo_values::domain::plan_parts::types::{CapturePattern, LiteralText, ParameterId};
    use ryo_values::domain::query_operators::types::{LineOutput, MaxLine, TabWidth, TextOutput};
    let from = Input::Step(StepId(0));
    let text = |written: &str| LiteralText(written.into());
    let matches = |span: u32| Operation::SourceMatches {
        from,
        span: ColumnIndex(span),
        base_line: None,
        pattern: text("x"),
        capture: None,
        capture_text: TextOutput::Omitted,
        capture_line: LineOutput::Omitted,
    };
    let render = |node: u32| Operation::RenderSyntax {
        from,
        node: ColumnIndex(node),
        context: ryo_language::domain::semantics::types::RenderContext::PositionalArgument,
    };
    let fits = |texts: u32, span: u32| Operation::FitsRendered {
        from,
        texts: vec![ColumnIndex(texts)],
        literals: vec![],
        span: ColumnIndex(span),
        max_line: MaxLine(80),
        tab_width: TabWidth(4),
    };
    let capture = |path: u32| Operation::Capture {
        from,
        path: ColumnIndex(path),
        pattern: CapturePattern("{x}".into()),
    };
    let replace = |column: u32, pattern: &str| Operation::Replace {
        from,
        text: ColumnIndex(column),
        pattern: text(pattern),
        replacement: text("b"),
    };
    vec![
        ("capture-col99", capture(99)),
        ("capture-num", capture(1)),
        (
            "rank-col99",
            Operation::Rank {
                from,
                column: ColumnIndex(99),
                sets: ParameterId(0),
            },
        ),
        (
            "words-col99",
            Operation::Words {
                from,
                text: ColumnIndex(99),
            },
        ),
        (
            "words-num",
            Operation::Words {
                from,
                text: ColumnIndex(1),
            },
        ),
        ("replace-col99", replace(99, "a")),
        ("replace-num", replace(1, "a")),
        ("replace-badre", replace(0, "(")),
        (
            "spanbetween-col99",
            Operation::SpanBetween {
                from,
                start: ColumnIndex(99),
                end: ColumnIndex(0),
            },
        ),
        (
            "spanbetween-text",
            Operation::SpanBetween {
                from,
                start: ColumnIndex(0),
                end: ColumnIndex(0),
            },
        ),
        ("sourcematches-col99", matches(99)),
        ("sourcematches-text", matches(0)),
        ("render-col99", render(99)),
        ("render-text", render(0)),
        ("fits-col99", fits(99, 1)),
        ("fits-spantext", fits(0, 0)),
        (
            "spanbounds-col99",
            Operation::SpanBounds {
                from,
                span: ColumnIndex(99),
            },
        ),
        (
            "spanbounds-text",
            Operation::SpanBounds {
                from,
                span: ColumnIndex(0),
            },
        ),
    ]
}

/// A projection of step 1's `columns`.
pub(super) fn project(columns: &[u32]) -> Operation {
    Operation::Project {
        from: Input::Step(StepId(1)),
        columns: columns.iter().copied().map(ColumnIndex).collect(),
        distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
    }
}

/// What rule 0 answers, collected, over the hosted `text(value: Text, n: Number)` with one row or none, `operation` over it,
/// and `reader` of `operation` after it, when there is one, emitted.
pub(super) fn row_by_row(
    operation: Operation,
    populated: bool,
    reader: Option<Operation>,
) -> String {
    use ryo_values::domain::plan_parts::types::HostedNumber;
    let mut input = authored_request(1);
    input.hosted = vec![HostRelation {
        name: HostName("text".into()),
        columns: vec![
            HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Text,
            },
            HostColumn {
                name: HostColumnName("n".into()),
                kind: HostKind::Number,
            },
        ],
        rows: if populated {
            vec![HostRow(vec![
                HostCell::Text("x".into()),
                HostCell::Number(Integer(1)),
            ])]
        } else {
            vec![]
        },
    }];
    let mut steps = vec![
        step(Operation::Hosted {
            relation: HostedNumber(0),
        }),
        step(operation),
    ];
    steps.extend(reader.map(step));
    let emitted = StepId(u32::try_from(steps.len() - 1).expect("a few steps"));
    boundary_fixture::fixtures::plans(&mut input)[0].steps = steps;
    boundary_fixture::fixtures::plans(&mut input)[0].emit.step = emitted;
    described(evaluate(input, &mut TableFacts::new(&[])))
}
