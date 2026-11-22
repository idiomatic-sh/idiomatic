//! The plan whose counted work the growth harness holds for each operation, over a file of words `w0 w1 ..` of one sentence,
//! each word once: every plan reads rows that grow with the file and none that grow faster, so work that grows faster than its
//! rows is the engine's. The kinds whose case waits for the lowering or the executor are in `pending`.

use super::parts::{at, compared, joined, of, plan, projected, step, words, words_where};
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_orders::types::{GroupEnd, OrderBound};
use ryo_values::domain::plan_parts::types::{
    CapturePattern, Direction, HostedNumber, Input, LiteralText, Operand, ParameterId,
    ParameterName, QueryLiteral, RepeatedRows, Side, StepId, StepScope, TableKey, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::{ParameterType, PlanParameter};
use ryo_values::domain::query_operators::types::{
    LineOutput, MaxLine, OperationKind, TabWidth, TextOutput,
};
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::written_rows::types::{RowsSource, WrittenRow, WrittenRows};

/// The words of the file (step 0 of every growth plan): parent, child, ordinal, text, file.
const WORDS: Input = Input::Step(StepId(0));

/// The words with where each is: parent, child, ordinal, text, file, span. Step 1 of the plans that read a span.
fn placed_words() -> PlanStep {
    joined(
        Input::Relation(Relation::SyntaxChild),
        Input::Relation(Relation::SyntaxTree),
        (&[1], &[0]),
        vec![
            of(Side::Left, 0),
            of(Side::Left, 1),
            of(Side::Left, 2),
            of(Side::Right, 2),
            of(Side::Left, 4),
            of(Side::Right, 4),
        ],
    )
}

/// A plan of the words and `rest`, emitting the last step's rows.
fn over_words(rest: Vec<PlanStep>) -> Plan {
    let mut steps = vec![words()];
    steps.extend(rest);
    plan(steps)
}

/// The first word of the file: the word whose ordinal is 0.
fn first_word() -> PlanStep {
    words_where(compared(Test::Equal, 2, 0))
}

/// The sentence and its file, once: the words' parent and file with repeats dropped.
fn sentence() -> PlanStep {
    step(Operation::Project {
        from: WORDS,
        columns: vec![at(0), at(4)],
        distinct: RepeatedRows::Dropped,
    })
}

/// The plan of the graph operations' cases: the lattice of one pair of states (0 below 1), every edge of the sentence to a
/// word holding the lower state before and the upper after, and a worklist from the first word's sentence in the lower state.
fn worklist() -> Plan {
    over_words(vec![
        first_word(),
        words_where(compared(Test::Equal, 2, 1)),
        joined(
            Input::Step(StepId(1)),
            Input::Step(StepId(2)),
            (&[4], &[4]),
            vec![of(Side::Left, 2), of(Side::Right, 2), of(Side::Left, 4)],
        ),
        joined(
            WORDS,
            Input::Step(StepId(3)),
            (&[4], &[2]),
            vec![
                of(Side::Left, 0),
                of(Side::Left, 1),
                of(Side::Left, 2),
                of(Side::Left, 4),
                of(Side::Right, 0),
                of(Side::Right, 1),
            ],
        ),
        step(Operation::Worklist {
            from: Input::Step(StepId(1)),
            edges: Input::Step(StepId(4)),
            lattice: Input::Step(StepId(3)),
            root: at(0),
            state: at(2),
            source: at(0),
            destination: at(1),
            order: at(2),
            before: at(4),
            after: at(5),
            lower: at(0),
            upper: at(1),
            root_keys: vec![at(4)],
            edge_keys: vec![at(3)],
        }),
    ])
}

/// The plan whose counted work the growth harness holds for an operation: one arm for each kind, so a new operation does not
/// compile without its growth case.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::fixtures::growth::growth_plan;
/// use ryo_values::domain::query_operators::types::OperationKind;
///
/// assert!(OperationKind::ALL.iter().all(|kind| !growth_plan(*kind).steps.is_empty()));
/// ```
#[must_use]
pub fn growth_plan(kind: OperationKind) -> Plan {
    let text = |written: &str| LiteralText(written.into());
    match kind {
        OperationKind::Select => {
            over_words(vec![words_where(compared(Test::GreaterThanOrEqual, 2, 0))])
        }
        OperationKind::Project => over_words(vec![projected(0, &[1, 3, 4])]),
        OperationKind::Extend => over_words(vec![step(Operation::Extend {
            from: WORDS,
            values: vec![Operand::Literal(QueryLiteral::Number(Integer(1)))],
        })]),
        OperationKind::Join => over_words(vec![joined(
            WORDS,
            WORDS,
            (&[1], &[1]),
            vec![of(Side::Left, 1), of(Side::Right, 3), of(Side::Left, 4)],
        )]),
        OperationKind::Union => over_words(vec![
            first_word(),
            step(Operation::Union {
                left: WORDS,
                right: Input::Step(StepId(1)),
            }),
        ]),
        OperationKind::Intersection => over_words(vec![
            projected(0, &[0, 1, 2, 3, 4]),
            step(Operation::Intersection {
                left: WORDS,
                right: Input::Step(StepId(1)),
            }),
        ]),
        OperationKind::Difference => over_words(vec![
            first_word(),
            step(Operation::Difference {
                left: WORDS,
                right: Input::Step(StepId(1)),
            }),
        ]),
        OperationKind::Antijoin => over_words(vec![
            first_word(),
            step(Operation::Antijoin {
                left: WORDS,
                right: Input::Step(StepId(1)),
                left_on: vec![at(1)],
                right_on: vec![at(1)],
            }),
        ]),
        OperationKind::GroupCount => over_words(vec![step(Operation::GroupCount {
            from: WORDS,
            keys: vec![at(4)],
            distinct: None,
        })]),
        OperationKind::GroupDigest => over_words(vec![step(Operation::GroupDigest {
            from: WORDS,
            keys: vec![at(4)],
            over: at(3),
        })]),
        OperationKind::GroupBound => over_words(vec![step(Operation::GroupBound {
            from: WORDS,
            keys: vec![at(4)],
            order: at(2),
            end: GroupEnd::Greatest,
        })]),
        OperationKind::GroupOrdinal => over_words(vec![step(Operation::GroupOrdinal {
            from: WORDS,
            keys: vec![at(4)],
            order: at(2),
        })]),
        OperationKind::Capture => over_words(vec![step(Operation::Capture {
            from: WORDS,
            path: at(3),
            pattern: CapturePattern("w{number}".into()),
        })]),
        OperationKind::Rank => {
            let mut ranked = over_words(vec![step(Operation::Rank {
                from: WORDS,
                column: at(3),
                sets: ParameterId(0),
            })]);
            ranked.parameters = vec![PlanParameter {
                name: ParameterName("sets".into()),
                kind: ParameterType::TextSets,
            }];
            ranked
        }
        OperationKind::Replace => over_words(vec![step(Operation::Replace {
            from: WORDS,
            text: at(3),
            pattern: text("w"),
            replacement: text("word "),
        })]),
        OperationKind::Words => over_words(vec![step(Operation::Words {
            from: WORDS,
            text: at(3),
        })]),
        OperationKind::OrderedMatch => over_words(vec![step(Operation::OrderedMatch {
            left: WORDS,
            right: WORDS,
            left_on: vec![at(4)],
            right_on: vec![at(4)],
            left_order: at(2),
            right_order: at(2),
            output: vec![of(Side::Left, 1), of(Side::Right, 1), of(Side::Left, 4)],
        })]),
        OperationKind::AsOf => over_words(vec![step(Operation::AsOf {
            left: WORDS,
            right: WORDS,
            left_on: vec![at(4)],
            right_on: vec![at(4)],
            left_order: at(2),
            right_order: at(2),
            bound: OrderBound::Before,
            output: vec![of(Side::Left, 1), of(Side::Right, 1), of(Side::Left, 4)],
        })]),
        OperationKind::Axis => over_words(vec![step(Operation::Axis {
            from: WORDS,
            edges: Input::Relation(Relation::SyntaxChild),
            domain: Input::Relation(Relation::SyntaxTree),
            direction: Direction::Parent,
            id: at(1),
            parent: at(0),
            child: at(1),
            domain_id: at(0),
            domain_filter: Vec::new(),
            domain_view: Vec::new(),
            output: Vec::new(),
        })]),
        OperationKind::Closure => over_words(vec![
            projected(0, &[0, 1, 4]),
            step(Operation::Closure {
                edges: Input::Step(StepId(1)),
                keys: vec![at(2)],
                source: at(0),
                destination: at(1),
            }),
        ]),
        OperationKind::Follow => over_words(vec![
            projected(0, &[0, 1, 4]),
            projected(0, &[1, 1, 4]),
            step(Operation::Union {
                left: Input::Step(StepId(1)),
                right: Input::Step(StepId(2)),
            }),
            step(Operation::Follow {
                edges: Input::Step(StepId(3)),
                keys: vec![at(2)],
                source: at(0),
                destination: at(1),
            }),
        ]),
        OperationKind::Cycles => over_words(vec![
            projected(0, &[1, 1]),
            step(Operation::Cycles {
                edges: Input::Step(StepId(1)),
                source: at(0),
                destination: at(1),
            }),
        ]),
        OperationKind::OrderedDfs => over_words(vec![
            sentence(),
            step(Operation::OrderedDfs {
                from: Input::Step(StepId(1)),
                edges: WORDS,
                root: at(0),
                source: at(0),
                destination: at(1),
                order: at(2),
                edge: at(1),
                root_keys: vec![at(1)],
                edge_keys: vec![at(4)],
            }),
        ]),
        OperationKind::Worklist => worklist(),
        OperationKind::Rows => over_words(vec![
            PlanStep {
                written: Default::default(),
                operation: Operation::Rows {
                    source: RowsSource::Written(WrittenRows(vec![WrittenRow(vec![text("w0")])])),
                    keys: vec![TableKey("text".into())],
                },
                scope: StepScope::PerFile,
            },
            joined(
                WORDS,
                Input::Step(StepId(1)),
                (&[3], &[0]),
                vec![of(Side::Left, 1), of(Side::Left, 4)],
            ),
        ]),
        OperationKind::Hosted => plan(vec![
            step(Operation::Hosted {
                relation: HostedNumber(0),
            }),
            projected(0, &[0]),
        ]),
        OperationKind::SpanBetween => over_words(vec![
            placed_words(),
            step(Operation::SpanBetween {
                from: Input::Step(StepId(1)),
                start: at(5),
                end: at(5),
            }),
        ]),
        OperationKind::SpanBounds => over_words(vec![
            placed_words(),
            step(Operation::SpanBounds {
                from: Input::Step(StepId(1)),
                span: at(5),
            }),
        ]),
        OperationKind::SourceMatches => over_words(vec![
            placed_words(),
            step(Operation::SourceMatches {
                from: Input::Step(StepId(1)),
                span: at(5),
                base_line: None,
                pattern: text("w"),
                capture: None,
                capture_text: TextOutput::Omitted,
                capture_line: LineOutput::Omitted,
            }),
        ]),
        OperationKind::RenderSyntax => over_words(vec![step(Operation::RenderSyntax {
            from: WORDS,
            node: at(1),
            context: RenderContext::PositionalArgument,
        })]),
        OperationKind::FitsRendered => over_words(vec![
            placed_words(),
            step(Operation::FitsRendered {
                from: Input::Step(StepId(1)),
                texts: vec![at(3)],
                literals: vec![text(""), text("")],
                span: at(5),
                max_line: MaxLine(1_000_000),
                tab_width: TabWidth(8),
            }),
        ]),
    }
}
