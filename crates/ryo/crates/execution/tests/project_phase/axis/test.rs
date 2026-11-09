//! An axis and a join with reach made after the files walk the frontier tables of the syntax tree and answer what they
//! answer in each file; the owner of the project phase holds no graph cache, so neither asks it for one. An anti-join over
//! the self-join of syntax-child ordinals is no ordered existence, as the ordinals are read from the tree, and answers its
//! rows.
use super::super::*;
use super::fixtures::rows;
use ryo_values::domain::plan_parts::types::{
    ComparisonOrigin, Direction, LiteralText, Operand, PlanComparison, QueryLiteral, ReachLookup,
    RepeatedRows, Test,
};

/// The files every proof runs over.
const FILES: usize = 3;

fn step(scope: StepScope, operation: Operation) -> PlanStep {
    PlanStep { scope, operation }
}

/// The ids of every element of the syntax tree, in each file.
fn elements() -> PlanStep {
    step(
        StepScope::PerFile,
        Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        },
    )
}

/// The distinct ids of step 0, made at `scope`.
fn gathered(scope: StepScope) -> PlanStep {
    step(
        scope,
        Operation::Project {
            from: Input::Step(StepId(0)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        },
    )
}

/// The parent of each gathered element, made at `scope`.
fn axis(scope: StepScope) -> Vec<PlanStep> {
    vec![
        elements(),
        gathered(scope),
        step(
            scope,
            Operation::Axis {
                from: Input::Step(StepId(1)),
                edges: Input::Relation(Relation::SyntaxChild),
                domain: Input::Relation(Relation::SyntaxTree),
                direction: Direction::Parent,
                id: ColumnIndex(0),
                parent: ColumnIndex(0),
                child: ColumnIndex(1),
                domain_id: ColumnIndex(0),
                domain_filter: vec![],
                domain_view: vec![],
                output: vec![],
            },
        ),
    ]
}

/// The elements each gathered element reaches below it, made at `scope`.
fn reach(scope: StepScope) -> Vec<PlanStep> {
    vec![
        elements(),
        gathered(scope),
        step(
            scope,
            Operation::Join {
                left: Input::Step(StepId(1)),
                right: Input::Relation(Relation::SyntaxChild),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(1),
                }],
                filter: vec![],
                view: vec![],
                reach: Some(ReachLookup {
                    source: ColumnIndex(0),
                    destination: ColumnIndex(1),
                    keys: vec![],
                    columns: vec![ColumnIndex(0), ColumnIndex(1)],
                }),
            },
        ),
    ]
}

#[test]
fn an_axis_after_the_files_answers_the_parents_it_answers_in_each_file() {
    let in_each_file =
        rows(axis(StepScope::PerFile), StepId(2), FILES).expect("the axis in each file");
    let after_the_files =
        rows(axis(StepScope::CrossFile), StepId(2), FILES).expect("the axis after the files");
    // Each file's two tokens have the root as their parent; the root has none.
    assert_eq!(in_each_file.len(), 2 * FILES);
    assert_eq!(after_the_files, in_each_file);
}

#[test]
fn a_join_with_reach_after_the_files_answers_what_it_reaches_in_each_file() {
    let in_each_file =
        rows(reach(StepScope::PerFile), StepId(2), FILES).expect("the reach in each file");
    let after_the_files =
        rows(reach(StepScope::CrossFile), StepId(2), FILES).expect("the reach after the files");
    // Each file's root reaches its two tokens; a token reaches nothing.
    assert_eq!(in_each_file.len(), 2 * FILES);
    assert_eq!(after_the_files, in_each_file);
}

#[test]
fn an_anti_join_over_the_self_join_of_syntax_child_ordinals_answers_the_first_token() {
    let file = StepScope::PerFile;
    let steps = vec![
        // The tokens.
        step(
            file,
            Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("token".into()))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: vec![ColumnIndex(0)],
            },
        ),
        // Each token with its parent and its ordinal under it.
        step(
            file,
            Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Relation(Relation::SyntaxChild),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(1)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(2),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        ),
        // Every pair of tokens under one parent: the token, its parent, its ordinal and the other's.
        step(
            file,
            Operation::Join {
                left: Input::Step(StepId(1)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(1)],
                right_on: vec![ColumnIndex(1)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(1),
                    },
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(2),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(2),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        ),
        // The tokens with an earlier sibling.
        step(
            file,
            Operation::Select {
                from: Input::Step(StepId(2)),
                comparisons: vec![PlanComparison {
                    test: Test::LessThan,
                    left: Operand::Column(ColumnIndex(3)),
                    right: Operand::Column(ColumnIndex(2)),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: vec![],
            },
        ),
        step(
            file,
            Operation::Project {
                from: Input::Step(StepId(3)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            },
        ),
        // The tokens with none: the first under its parent.
        step(
            file,
            Operation::Antijoin {
                left: Input::Step(StepId(1)),
                right: Input::Step(StepId(4)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
        ),
    ];
    let first = rows(steps, StepId(5), FILES).expect("the anti-join answers");
    assert_eq!(
        first.len(),
        FILES,
        "one first token in each file: {first:?}"
    );
    assert!(
        first.iter().all(|row| row.ends_with("Number(Integer(0))]")),
        "the first token is the one at ordinal 0: {first:?}"
    );
}
