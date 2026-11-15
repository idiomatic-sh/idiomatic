//! Combined cases of the text kinds: a resource a kind acquires at its first row, the order of two failing inputs of an ordered
//! match, and the source gate before the pattern, each answer recorded from the interpreter on the stack's base.
use super::super::fixtures::{
    COLLECTED, SpanCell, answered, answered_over, functions, over_a_shared_node,
};
use crate::boundary_fixture::fixtures::{FileRun, authored_request, corpus, run_file, source};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase as _;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_language::domain::symbols::types::Symbols;
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

/// Two rules share one render node, and the semantic session cannot render its syntax: each rule is refused with the engine's
/// reason, the second too, which reads the node the first one ran.
#[test]
fn a_shared_render_refuses_every_rule_with_the_engine_s_reason() {
    use ryo_execution::inbound::use_cases::execution::interface::{
        OperatingError, SemanticLookup, SemanticSupport,
    };
    use ryo_language::domain::semantics::types::{RenderContext, RenderOutcome, RenderRequest};
    #[derive(Debug)]
    struct Unrenderable;
    impl SemanticLookup for Unrenderable {
        fn render_batch(
            &self,
            requests: &[RenderRequest],
            _: ryo_values::domain::files::types::SourceView<'_>,
        ) -> Result<Vec<RenderOutcome>, OperatingError> {
            Ok(requests
                .iter()
                .map(|_| {
                    RenderOutcome::Unsupported(ryo_relations::domain::terms::types::UnknownReason(
                        24,
                    ))
                })
                .collect())
        }
    }
    let render = Operation::RenderSyntax {
        from: Input::Step(StepId(0)),
        node: ColumnIndex(0),
        context: RenderContext::PositionalArgument,
    };
    let program = over_a_shared_node(&[Some(render.clone()), Some(render)]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let rules = run_file(
        &request,
        FileRun {
            semantic: SemanticSupport::Supplied(&Unrenderable),
            ..FileRun::of(&[COLLECTED, COLLECTED], &elements)
        },
        &mut Symbols::default(),
        &mut facts,
    );
    request.finish();
    let shown = answered(rules);
    assert_eq!(
        shown
            .lines()
            .filter(|line| line.contains("cannot render this syntax (reason 24)"))
            .count(),
        2,
        "both rules answer the engine's reason: {shown}"
    );
}

/// The session's answer for each role: a role it cannot render (reason 24 or 25), and a role whose batch comes back one
/// answer short.
#[derive(Debug)]
struct ByRole;

impl ryo_execution::inbound::use_cases::execution::interface::SemanticLookup for ByRole {
    fn render_batch(
        &self,
        requests: &[ryo_language::domain::semantics::types::RenderRequest],
        _: ryo_values::domain::files::types::SourceView<'_>,
    ) -> Result<
        Vec<ryo_language::domain::semantics::types::RenderOutcome>,
        ryo_execution::inbound::use_cases::execution::interface::OperatingError,
    > {
        use ryo_language::domain::semantics::types::{RenderContext, RenderOutcome};
        use ryo_relations::domain::terms::types::UnknownReason;
        Ok(requests
            .iter()
            .filter_map(|request| match request.context {
                RenderContext::PositionalArgument => {
                    Some(RenderOutcome::Unsupported(UnknownReason(24)))
                }
                RenderContext::AttributeReceiver => {
                    Some(RenderOutcome::Unsupported(UnknownReason(25)))
                }
                RenderContext::AssignmentTarget => None,
            })
            .collect())
    }
}

/// A rendering of Function's node (step 0) in `context`.
fn rendered_as(context: ryo_language::domain::semantics::types::RenderContext) -> Operation {
    Operation::RenderSyntax {
        from: Input::Step(StepId(0)),
        node: ColumnIndex(0),
        context,
    }
}

/// The answers over file 0 of one rule per entry of `rules`, each a select of Function's node followed by its steps, its last
/// step emitted, rendered through `ByRole`.
fn rendered_rules(rules: &[Vec<Operation>]) -> String {
    use ryo_execution::inbound::use_cases::execution::interface::SemanticSupport;
    let mut input = authored_request(rules.len());
    for (plan, steps) in crate::boundary_fixture::fixtures::plans(&mut input)
        .iter_mut()
        .zip(rules)
    {
        plan.steps.truncate(1);
        plan.steps.extend(steps.iter().map(|operation| PlanStep {
            scope: StepScope::PerFile,
            operation: operation.clone(),
        }));
        plan.emit.step = StepId(u32::try_from(steps.len()).unwrap_or(u32::MAX));
    }
    use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
    let program = crate::boundary_fixture::fixtures::engine()
        .prepare(input)
        .expect("prepare the rendering rules");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let intents = vec![COLLECTED; rules.len()];
    let rules = run_file(
        &request,
        FileRun {
            semantic: SemanticSupport::Supplied(&ByRole),
            ..FileRun::of(&intents, &elements)
        },
        &mut Symbols::default(),
        &mut facts,
    );
    request.finish();
    answered(rules)
}

/// The union of steps 1 and 2.
fn union_of_both() -> Operation {
    Operation::Union {
        left: Input::Step(StepId(1)),
        right: Input::Step(StepId(2)),
    }
}

/// Rule 0 renders A (unrenderable) and B (a short batch) and answers A's refusal, its union's left; rule 1 reads B, held, and
/// answers B's own refusal: failed with the batch's shape.
#[test]
fn a_reader_of_a_short_batch_answers_its_shape_after_an_unrenderable_one() {
    use ryo_language::domain::semantics::types::RenderContext::{
        AssignmentTarget, PositionalArgument,
    };
    let shown = rendered_rules(&[
        vec![
            rendered_as(PositionalArgument),
            rendered_as(AssignmentTarget),
            union_of_both(),
        ],
        vec![rendered_as(AssignmentTarget)],
    ]);
    let lines: Vec<&str> = shown.lines().collect();
    assert!(
        lines.first().is_some_and(
            |line| line.starts_with("rule 0: unknown SemanticIncomplete")
                && line.contains("(reason 24)")
        ) && lines
            .get(1)
            .is_some_and(|line| line.starts_with("rule 1: failed SemanticBatchShape")),
        "each rule answers the refusal of the node it reads: {shown}"
    );
}

/// The reverse order of categories: rule 0's union reads the unrenderable node, then the short batch; rule 1 reads the
/// unrenderable node, held, and answers its refusal, not the short batch's.
#[test]
fn a_reader_of_an_unrenderable_node_answers_its_reason_after_a_short_batch() {
    use ryo_language::domain::semantics::types::RenderContext::{
        AssignmentTarget, PositionalArgument,
    };
    let shown = rendered_rules(&[
        vec![
            rendered_as(PositionalArgument),
            rendered_as(AssignmentTarget),
            union_of_both(),
        ],
        vec![rendered_as(PositionalArgument)],
    ]);
    let lines: Vec<&str> = shown.lines().collect();
    assert!(
        lines.get(1).is_some_and(
            |line| line.starts_with("rule 1: unknown SemanticIncomplete")
                && line.contains("(reason 24)")
        ),
        "the reader of the unrenderable node answers its reason: {shown}"
    );
}

/// Two unrenderable nodes of other reasons: each reader answers its own node's reason.
#[test]
fn each_reader_answers_the_reason_of_the_node_it_reads() {
    use ryo_language::domain::semantics::types::RenderContext::{
        AttributeReceiver, PositionalArgument,
    };
    let shown = rendered_rules(&[
        vec![
            rendered_as(PositionalArgument),
            rendered_as(AttributeReceiver),
            union_of_both(),
        ],
        vec![rendered_as(AttributeReceiver)],
        vec![rendered_as(PositionalArgument)],
    ]);
    let lines: Vec<&str> = shown.lines().collect();
    assert!(
        lines
            .first()
            .is_some_and(|line| line.contains("(reason 24)"))
            && lines
                .get(1)
                .is_some_and(|line| line.contains("(reason 25)"))
            && lines
                .get(2)
                .is_some_and(|line| line.contains("(reason 24)")),
        "each rule answers its own node's reason: {shown}"
    );
}
