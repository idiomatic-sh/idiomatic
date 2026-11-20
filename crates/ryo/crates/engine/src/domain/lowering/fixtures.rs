//! The test support of the `lowering` concept: the plans the engine's seam scenarios name, built by hand (the engine reads no query
//! text). Each is the smallest plan of its shape; the lowering's track makes each faithful to the shape it names.

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_orders::types::{GroupEnd, OrderBound};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Heading, Input, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepId, StepScope, TableKey, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::{Emit, Plan};
use ryo_values::domain::written_rows::types::{RowsSource, WrittenRow, WrittenRows};

/// A step that keeps every row of `from`.
fn kept(from: Input) -> PlanStep {
    PlanStep {
        operation: Operation::Select {
            from,
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    }
}

/// A plan of `steps` emitting the last one's rows.
fn plan(steps: Vec<PlanStep>) -> Plan {
    let last = u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX);
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: StepId(last),
            subject: ColumnIndex(0),
            key: vec![ColumnIndex(0)],
            headings: vec![Heading("node".into())],
        },
        frontier: Vec::new(),
    }
}

/// A per-file step of `operation`.
fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

/// The column at `place`.
const fn at(place: u32) -> ColumnIndex {
    ColumnIndex(place)
}

/// A column of one side of a join.
const fn of(side: Side, place: u32) -> JoinColumn {
    JoinColumn {
        side,
        column: ColumnIndex(place),
    }
}

/// A plain join of two inputs on equal keys.
fn joined(left: Input, right: Input, on: (&[u32], &[u32]), output: Vec<JoinColumn>) -> PlanStep {
    step(Operation::Join {
        left,
        right,
        left_on: on.0.iter().copied().map(ColumnIndex).collect(),
        right_on: on.1.iter().copied().map(ColumnIndex).collect(),
        output,
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    })
}

/// The words of a file, one row for each child of its sentence: parent, child, ordinal, text, file. Step 0 of the scenes below.
fn words() -> PlanStep {
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
        ],
    )
}

/// The words (step 0) whose text is `text`, keeping `kept` of their columns; none named keeps every column.
fn words_written(text: &str, kept: &[u32]) -> PlanStep {
    step(Operation::Select {
        from: Input::Step(StepId(0)),
        comparisons: vec![PlanComparison {
            test: Test::Equal,
            left: Operand::Column(at(3)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: kept.iter().copied().map(ColumnIndex).collect(),
    })
}

/// An as-of of two steps of words by ordinal, answering the left word, the right word and the file.
fn as_of(left: (u32, &[u32]), right: (u32, &[u32], u32), bound: OrderBound) -> PlanStep {
    step(Operation::AsOf {
        left: Input::Step(StepId(left.0)),
        right: Input::Step(StepId(right.0)),
        left_on: left.1.iter().copied().map(ColumnIndex).collect(),
        right_on: right.1.iter().copied().map(ColumnIndex).collect(),
        left_order: at(2),
        right_order: at(right.2),
        bound,
        output: vec![of(Side::Left, 1), of(Side::Right, 1), of(Side::Left, 4)],
    })
}

/// The plans of the operations' scan scenarios, each over the words of the scanned file (step 0); none for another name.
fn operation_scene(scene: &str) -> Option<Plan> {
    let words_of = Input::Step(StepId(0));
    Some(match scene {
        // Each word with the word of the same text latest before it: two key groups in a file of two texts.
        "as-of latest before" => plan(vec![
            words(),
            as_of((0, &[3, 4]), (0, &[3, 4], 2), OrderBound::Before),
        ]),
        // The right rows are each word beside each word of its text, ordered by the first word's ordinal: words of one text
        // tie at each order, and every tied row answers.
        "as-of tied events" => plan(vec![
            words(),
            joined(
                words_of,
                words_of,
                (&[3, 4], &[3, 4]),
                vec![of(Side::Left, 2), of(Side::Right, 1), of(Side::Left, 4)],
            ),
            as_of((0, &[4]), (1, &[2], 0), OrderBound::Before),
        ]),
        // Each word `a` with the word `b` earliest after it.
        "as-of earliest after" => plan(vec![
            words(),
            words_written("a", &[]),
            words_written("b", &[]),
            as_of((1, &[4]), (2, &[4], 2), OrderBound::After),
        ]),
        // Each word beside each word of its text, by the other word's ordinal: the greatest ordinal of a text is held by one
        // row for each of its words.
        "group-max ties" => plan(vec![
            words(),
            joined(
                words_of,
                words_of,
                (&[3, 4], &[3, 4]),
                vec![
                    of(Side::Left, 1),
                    of(Side::Right, 2),
                    of(Side::Left, 3),
                    of(Side::Left, 4),
                ],
            ),
            step(Operation::GroupBound {
                from: Input::Step(StepId(1)),
                keys: vec![at(2), at(3)],
                order: at(1),
                end: GroupEnd::Greatest,
            }),
        ]),
        // The words placed by ordinal within the file, and the ones whose place is 0: the first.
        "group-ordinal from zero" => plan(vec![
            words(),
            step(Operation::GroupOrdinal {
                from: words_of,
                keys: vec![at(4)],
                order: at(2),
            }),
            step(Operation::Select {
                from: Input::Step(StepId(1)),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(at(5)),
                    right: Operand::Literal(QueryLiteral::Number(
                        ryo_relations::domain::relations::types::Integer(0),
                    )),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            }),
        ]),
        // The sentence points to each of its words, and each word `a` to itself: the sentence reaches an `a` and an `a`
        // answers itself; the walk to any other word runs out of edges.
        "follow fixed points" => plan(vec![
            words(),
            step(Operation::Project {
                from: words_of,
                columns: vec![at(0), at(1), at(4)],
                distinct: RepeatedRows::Kept,
            }),
            words_written("a", &[1, 1, 4]),
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
        // Two rows written in the query: made once for the run, after the files, so the scan of a file finds none of them.
        "written rows" => plan(vec![PlanStep {
            operation: Operation::Rows {
                source: RowsSource::Written(WrittenRows(vec![
                    WrittenRow(vec![LiteralText("a".into())]),
                    WrittenRow(vec![LiteralText("b".into())]),
                ])),
                keys: vec![TableKey("text".into())],
            },
            scope: StepScope::CrossFile,
        }]),
        _ => return None,
    })
}

/// The plan a seam scenario names; an unknown name is the one tree rule.
#[must_use]
pub fn named(scene: &str) -> Plan {
    if let Some(plan) = operation_scene(scene) {
        return plan;
    }
    match scene {
        // An antijoin whose right side reads a variable only the left binds.
        "unbound correlated sub-plan" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            kept(Input::Relation(Relation::SyntaxChild)),
            PlanStep {
                operation: Operation::Antijoin {
                    left: Input::Step(StepId(0)),
                    right: Input::Step(StepId(1)),
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(1)],
                },
                scope: StepScope::PerFile,
            },
        ]),
        // A chain longer than the lowering's 4,096-instruction budget.
        "program over budget" => plan(
            std::iter::once(kept(Input::Relation(Relation::SyntaxTree)))
                .chain((0..4_096_u32).map(|step| kept(Input::Step(StepId(step)))))
                .collect(),
        ),
        // A rule reading two relations, one of which no enabled program reads in the file.
        "two relations, one read" => plan(vec![
            kept(Input::Relation(Relation::SyntaxError)),
            kept(Input::Relation(Relation::Function)),
        ]),
        // A rule reading the relation the fixture language fills only in part.
        "partial relation" => plan(vec![kept(Input::Relation(Relation::SyntaxError))]),
        _ => plan(vec![kept(Input::Relation(Relation::SyntaxTree))]),
    }
}
