//! Combined cases of the text kinds: a resource a kind acquires at its first row, the order of two failing inputs of an ordered
//! match, and the source gate before the pattern, each answer recorded from the interpreter on the stack's base.
use super::super::fixtures::{SpanCell, answered_over, functions};
use crate::boundary_fixture::fixtures::authored_request;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use std::fmt::Write as _;

/// A rank whose sets nothing binds refuses only once a row reaches it: no rows, no refusal.
#[test]
fn an_unbound_rank_over_no_rows() {
    use ryo_values::domain::plan_parts::types::{ParameterId, ParameterName};
    use ryo_values::domain::query_bindings::types::{ParameterType, PlanParameter};
    let mut shown = String::new();
    for count in [0, 1] {
        let mut input = authored_request(1);
        let plan = &mut crate::boundary_fixture::fixtures::plans(&mut input)[0];
        plan.parameters.push(PlanParameter {
            name: ParameterName("sets".into()),
            kind: ParameterType::TextSets,
        });
        plan.steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::Function),
                    comparisons: vec![],
                    kept: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Rank {
                    from: Input::Step(StepId(0)),
                    column: ColumnIndex(1),
                    sets: ParameterId(0),
                },
            },
        ];
        plan.emit.step = StepId(1);
        let table = functions(SpanCell::FirstByte, count);
        write!(
            shown,
            "{count} rows: {}",
            answered_over(input, &table, b"abc")
        )
        .unwrap();
    }
    assert_eq!(
        shown,
        "0 rows: rule 0: complete Table { kinds: [Node, Symbol, File, Span, Number], cells: [], count: RowCount(0) }; ran, 0 captures\nfile: finished\n1 rows: rule 0: failed Unbound; unreached\nfile: unfinished\n",
        "the answers main gave"
    );
}

/// Both inputs of an ordered match fail on their first row: the left input's fault is answered.
#[test]
fn ordered_match_over_two_failing_inputs() {
    let mut input = authored_request(1);
    let plan = &mut crate::boundary_fixture::fixtures::plans(&mut input)[0];
    plan.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::SpanBounds {
            from: Input::Step(StepId(0)),
            span: ColumnIndex(98),
        },
    });
    plan.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::SpanBounds {
            from: Input::Step(StepId(0)),
            span: ColumnIndex(99),
        },
    });
    plan.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::OrderedMatch {
            left: Input::Step(StepId(1)),
            right: Input::Step(StepId(2)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            left_order: ColumnIndex(1),
            right_order: ColumnIndex(1),
            output: vec![],
        },
    });
    plan.emit.step = StepId(3);
    assert_eq!(
        answered_over(input, &functions(SpanCell::FirstByte, 1), b"abc"),
        "rule 0: failed Table(NoSuchColumn { index: 98 }); unreached\nfile: unfinished\n",
        "the answers main gave"
    );
}

/// An absent span gives no rows and compiles nothing, so an invalid pattern refuses only a row with a span.
#[test]
fn source_matches_absent_span_with_an_invalid_pattern() {
    use ryo_values::domain::plan_parts::types::LiteralText;
    use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
    let mut shown = String::new();
    for span in [SpanCell::Absent, SpanCell::FirstByte] {
        let mut input = authored_request(1);
        let plan = &mut crate::boundary_fixture::fixtures::plans(&mut input)[0];
        plan.steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::Function),
                    comparisons: vec![],
                    kept: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::SourceMatches {
                    from: Input::Step(StepId(0)),
                    span: ColumnIndex(3),
                    base_line: None,
                    pattern: LiteralText("[".into()),
                    capture: None,
                    capture_text: TextOutput::Omitted,
                    capture_line: LineOutput::Omitted,
                },
            },
        ];
        plan.emit.step = StepId(1);
        write!(
            shown,
            "{span:?}: {}",
            answered_over(input, &functions(span, 1), b"abc")
        )
        .unwrap();
    }
    assert_eq!(
        shown,
        "Absent: rule 0: complete Table { kinds: [Node, Symbol, File, Span, Span], cells: [], count: RowCount(0) }; ran, 0 captures\nfile: finished\nFirstByte: rule 0: failed InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" }; unreached\nfile: unfinished\n",
        "the answers main gave"
    );
}
