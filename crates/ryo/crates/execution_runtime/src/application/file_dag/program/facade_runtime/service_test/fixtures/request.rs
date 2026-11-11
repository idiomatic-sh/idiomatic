//! The authored plan and rule each parity scenario submits.
use super::scenario::ProgramShape;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, JoinColumn, LiteralText, Operand, PlanComparison, QueryLiteral, RepeatedRows,
    Side, StepId, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
pub(crate) fn plan(scenario: ProgramShape) -> Plan {
    let mut steps = vec![PlanStep {
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::SyntaxTree),
            comparisons: if matches!(scenario, ProgramShape::OrdinaryFunctionJoin) {
                vec![]
            } else if matches!(
                scenario,
                ProgramShape::RowsOfWords | ProgramShape::DistinctKinds | ProgramShape::KeptKinds
            ) {
                vec![PlanComparison {
                    test: Test::NotEqual,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("file".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }]
            } else {
                vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }]
            },
            kept: if matches!(
                scenario,
                ProgramShape::DistinctKinds | ProgramShape::KeptKinds
            ) {
                vec![ColumnIndex(1)]
            } else {
                vec![ColumnIndex(0), ColumnIndex(1)]
            },
        },
        scope: StepScope::PerFile,
    }];
    if matches!(scenario, ProgramShape::RowsOfWords) {
        steps.push(PlanStep {
            operation: Operation::Select {
                from: PlanInput::Step(StepId(0)),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![ColumnIndex(0)],
            },
            scope: StepScope::PerFile,
        });
    }
    if matches!(
        scenario,
        ProgramShape::RowsOfWords | ProgramShape::DistinctKinds | ProgramShape::KeptKinds
    ) {
        steps.push(PlanStep {
            operation: Operation::Project {
                from: PlanInput::Step(StepId((steps.len() - 1) as u32)),
                columns: vec![ColumnIndex(0)],
                distinct: if matches!(scenario, ProgramShape::KeptKinds) {
                    RepeatedRows::Kept
                } else {
                    RepeatedRows::Dropped
                },
            },
            scope: StepScope::PerFile,
        });
    }
    if matches!(scenario, ProgramShape::Counted) {
        steps.push(PlanStep {
            operation: Operation::GroupCount {
                from: PlanInput::Step(StepId(0)),
                keys: vec![ColumnIndex(1)],
                distinct: None,
            },
            scope: StepScope::PerFile,
        });
    }
    if matches!(scenario, ProgramShape::MissingRight) {
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxChild),
                comparisons: vec![],
                kept: vec![],
            },
        });
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Antijoin {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(1)],
            },
        });
    }
    if matches!(scenario, ProgramShape::OrdinaryFunctionJoin) {
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Relation(Relation::Function),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        });
    }
    if matches!(
        scenario,
        ProgramShape::NativeAntijoin
            | ProgramShape::NativeUnion
            | ProgramShape::NativeJoinSteps
            | ProgramShape::NativeJoinTree
    ) {
        let select = |comparisons, kept| PlanStep {
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxTree),
                comparisons,
                kept,
            },
            scope: StepScope::PerFile,
        };
        let text_is = |column, test, text: &str| PlanComparison {
            test,
            left: Operand::Column(ColumnIndex(column)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        };
        steps = match scenario {
            ProgramShape::NativeAntijoin | ProgramShape::NativeUnion => vec![
                select(
                    vec![text_is(1, Test::NotEqual, "file")],
                    vec![ColumnIndex(1)],
                ),
                select(vec![text_is(1, Test::Equal, "word")], vec![ColumnIndex(1)]),
                PlanStep {
                    operation: if matches!(scenario, ProgramShape::NativeAntijoin) {
                        Operation::Antijoin {
                            left: PlanInput::Step(StepId(0)),
                            right: PlanInput::Step(StepId(1)),
                            left_on: vec![ColumnIndex(0)],
                            right_on: vec![ColumnIndex(0)],
                        }
                    } else {
                        Operation::Union {
                            left: PlanInput::Step(StepId(0)),
                            right: PlanInput::Step(StepId(1)),
                        }
                    },
                    scope: StepScope::PerFile,
                },
            ],
            ProgramShape::NativeJoinSteps => vec![
                select(
                    vec![text_is(1, Test::Equal, "word")],
                    vec![ColumnIndex(0), ColumnIndex(1)],
                ),
                select(
                    vec![text_is(2, Test::Equal, "a")],
                    vec![ColumnIndex(0), ColumnIndex(1)],
                ),
                PlanStep {
                    operation: Operation::Join {
                        left: PlanInput::Step(StepId(0)),
                        right: PlanInput::Step(StepId(1)),
                        left_on: vec![ColumnIndex(1)],
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
                        ],
                        filter: vec![],
                        view: vec![],
                        reach: None,
                    },
                    scope: StepScope::PerFile,
                },
            ],
            ProgramShape::NativeJoinTree => vec![
                select(vec![text_is(1, Test::Equal, "other")], vec![ColumnIndex(0)]),
                PlanStep {
                    operation: Operation::Join {
                        left: PlanInput::Step(StepId(0)),
                        right: PlanInput::Relation(Relation::SyntaxTree),
                        left_on: vec![ColumnIndex(0)],
                        right_on: vec![ColumnIndex(0)],
                        output: vec![
                            JoinColumn {
                                side: Side::Left,
                                column: ColumnIndex(0),
                            },
                            JoinColumn {
                                side: Side::Right,
                                column: ColumnIndex(0),
                            },
                        ],
                        filter: vec![],
                        view: vec![],
                        reach: None,
                    },
                    scope: StepScope::PerFile,
                },
            ],
            _ => panic!("the fixture lists every case it builds"),
        };
    }
    let last = StepId((steps.len() - 1) as u32);
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        steps,
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: last,
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}
pub(crate) fn rule() -> RuleSpec {
    RuleSpec {
        id: RuleId("native-parity".into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId("native-parity".into()),
                text: "".into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    }
}
