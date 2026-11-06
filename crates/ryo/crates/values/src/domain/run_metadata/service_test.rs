//! Metadata preserves declared phases, bound-input capture policy and every feature's real input ordinal.
use super::service::from_plans;
use super::types::{CrossSteps, FeatureSource, FrontierCapture, ProjectRead};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    observations::types::{BoundObservationInput, ObservationBindingName},
    plan_parts::types::{ColumnIndex, Comparison, Direction, Input as PlanInput, LiteralText, Operand, QueryLiteral, RepeatedRows, StepId, Test, ObservationBindingId, StepScope},
    plans::types::{Emit, Feature, Operation, OperationKind, Plan, PlanStep},
};
use ryo_relations::domain::relations::types::Relation;
use std::borrow::Cow;
#[derive(Clone, Copy)]
enum Scenario {
    Empty,
    PerFile,
    Project,
    Observations,
    Features,
}
type Input = Scenario;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Metadata {
    Rule {
        reads_project: bool,
        has_cross_steps: bool,
    },
    Capture {
        input: usize,
        retained: bool,
    },
    Step {
        operation: OperationKind,
        scope: StepScope,
    },
    Feature {
        feature: Feature,
        source: FeatureSource,
    },
}
type Value = Cow<'static, [Metadata]>;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_plans_have_no_metadata"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("per_file_steps_do_not_claim_project_or_cross_file_work"),
        word: Word::Succeeds,
        input: || Scenario::PerFile,
        expect: Expect::Value(Cow::Borrowed(&[
            Metadata::Rule {
                reads_project: false,
                has_cross_steps: false,
            },
            Metadata::Step {
                operation: OperationKind::Select,
                scope: StepScope::PerFile,
            },
            Metadata::Feature {
                feature: Feature::Operation(OperationKind::Select),
                source: FeatureSource::AllInputs,
            },
            Metadata::Feature {
                feature: Feature::Relation(Relation::Function),
                source: FeatureSource::Input(0),
            },
        ])),
    },
    Case {
        name: CaseName("manifest_frontier_and_actual_cross_step_are_recorded_independently"),
        word: Word::Succeeds,
        input: || Scenario::Project,
        expect: Expect::Value(Cow::Borrowed(&[
            Metadata::Rule {
                reads_project: true,
                has_cross_steps: true,
            },
            Metadata::Capture {
                input: 0,
                retained: true,
            },
            Metadata::Capture {
                input: 1,
                retained: true,
            },
            Metadata::Step {
                operation: OperationKind::Select,
                scope: StepScope::PerFile,
            },
            Metadata::Feature {
                feature: Feature::Operation(OperationKind::Select),
                source: FeatureSource::AllInputs,
            },
            Metadata::Feature {
                feature: Feature::Relation(Relation::Function),
                source: FeatureSource::Input(0),
            },
            Metadata::Step {
                operation: OperationKind::Project,
                scope: StepScope::CrossFile,
            },
            Metadata::Feature {
                feature: Feature::Operation(OperationKind::Project),
                source: FeatureSource::AllInputs,
            },
        ])),
    },
    Case {
        name: CaseName("only_cross_file_bound_observation_frontiers_are_not_captured_per_file"),
        word: Word::Succeeds,
        input: || Scenario::Observations,
        expect: Expect::Value(Cow::Borrowed(&[
            Metadata::Rule {
                reads_project: false,
                has_cross_steps: true,
            },
            Metadata::Capture {
                input: 0,
                retained: true,
            },
            Metadata::Capture {
                input: 1,
                retained: false,
            },
            Metadata::Capture {
                input: 2,
                retained: true,
            },
            Metadata::Capture {
                input: 3,
                retained: true,
            },
            Metadata::Step {
                operation: OperationKind::Project,
                scope: StepScope::CrossFile,
            },
            Metadata::Feature {
                feature: Feature::Operation(OperationKind::Project),
                source: FeatureSource::AllInputs,
            },
        ])),
    },
    Case {
        name: CaseName(
            "comparison_direction_and_relation_features_keep_authored_order_and_input_slots",
        ),
        word: Word::Succeeds,
        input: || Scenario::Features,
        expect: Expect::Value(Cow::Borrowed(&[
            Metadata::Rule {
                reads_project: false,
                has_cross_steps: false,
            },
            Metadata::Step {
                operation: OperationKind::Select,
                scope: StepScope::PerFile,
            },
            Metadata::Feature {
                feature: Feature::Operation(OperationKind::Select),
                source: FeatureSource::AllInputs,
            },
            Metadata::Feature {
                feature: Feature::Family(Test::Equal),
                source: FeatureSource::AllInputs,
            },
            Metadata::Feature {
                feature: Feature::Relation(Relation::Identifier),
                source: FeatureSource::Input(0),
            },
            Metadata::Step {
                operation: OperationKind::Join,
                scope: StepScope::PerFile,
            },
            Metadata::Feature {
                feature: Feature::Operation(OperationKind::Join),
                source: FeatureSource::AllInputs,
            },
            Metadata::Feature {
                feature: Feature::Family(Test::Equal),
                source: FeatureSource::AllInputs,
            },
            Metadata::Feature {
                feature: Feature::Family(Test::GreaterThan),
                source: FeatureSource::AllInputs,
            },
            Metadata::Feature {
                feature: Feature::Relation(Relation::Function),
                source: FeatureSource::Input(1),
            },
            Metadata::Step {
                operation: OperationKind::Axis,
                scope: StepScope::PerFile,
            },
            Metadata::Feature {
                feature: Feature::Operation(OperationKind::Axis),
                source: FeatureSource::AllInputs,
            },
            Metadata::Feature {
                feature: Feature::Family(Test::Equal),
                source: FeatureSource::AllInputs,
            },
            Metadata::Feature {
                feature: Feature::Direction(Direction::Parent),
                source: FeatureSource::AllInputs,
            },
            Metadata::Feature {
                feature: Feature::Relation(Relation::SyntaxChild),
                source: FeatureSource::Input(1),
            },
            Metadata::Feature {
                feature: Feature::Relation(Relation::SyntaxTree),
                source: FeatureSource::Input(2),
            },
        ])),
    },
];
fn plan(steps: Vec<PlanStep>, frontier: Vec<PlanInput>) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier,
    }
}
fn select(from: Relation) -> PlanStep {
    PlanStep {
        operation: Operation::Select {
            from: PlanInput::Relation(from),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    }
}
fn project(from: PlanInput) -> PlanStep {
    PlanStep {
        operation: Operation::Project {
            from,
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
        scope: StepScope::CrossFile,
    }
}
fn comparison(test: Test) -> Comparison {
    Comparison {
        test,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("name".into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}
fn subject(input: Input) -> Result<Value, Failure> {
    let plans = match input {
        Scenario::Empty => Vec::new(),
        Scenario::PerFile => vec![plan(vec![select(Relation::Function)], Vec::new())],
        Scenario::Project => vec![plan(
            vec![
                select(Relation::Function),
                project(PlanInput::Step(StepId(0))),
            ],
            vec![
                PlanInput::Relation(Relation::PackageDependency),
                PlanInput::Step(StepId(0)),
            ],
        )],
        Scenario::Observations => {
            let mut plan = plan(
                vec![project(PlanInput::BoundObservation(ObservationBindingId(
                    1,
                )))],
                vec![
                    PlanInput::BoundObservation(ObservationBindingId(0)),
                    PlanInput::BoundObservation(ObservationBindingId(1)),
                    PlanInput::Relation(Relation::Identifier),
                    PlanInput::Step(StepId(0)),
                ],
            );
            plan.observation_inputs = vec![
                BoundObservationInput {
                    name: ObservationBindingName("per_file".into()),
                    columns: Vec::new(),
                    scope: StepScope::PerFile,
                },
                BoundObservationInput {
                    name: ObservationBindingName("cross_file".into()),
                    columns: Vec::new(),
                    scope: StepScope::CrossFile,
                },
            ];
            vec![plan]
        }
        Scenario::Features => vec![plan(
            vec![
                PlanStep {
                    operation: Operation::Select {
                        from: PlanInput::Relation(Relation::Identifier),
                        comparisons: vec![comparison(Test::Equal)],
                        kept: Vec::new(),
                    },
                    scope: StepScope::PerFile,
                },
                PlanStep {
                    operation: Operation::Join {
                        left: PlanInput::Step(StepId(0)),
                        right: PlanInput::Relation(Relation::Function),
                        left_on: vec![ColumnIndex(0)],
                        right_on: vec![ColumnIndex(0)],
                        output: Vec::new(),
                        filter: vec![comparison(Test::Equal), comparison(Test::GreaterThan)],
                        view: Vec::new(),
                        reach: None,
                    },
                    scope: StepScope::PerFile,
                },
                PlanStep {
                    operation: Operation::Axis {
                        from: PlanInput::Step(StepId(1)),
                        edges: PlanInput::Relation(Relation::SyntaxChild),
                        domain: PlanInput::Relation(Relation::SyntaxTree),
                        direction: Direction::Parent,
                        id: ColumnIndex(0),
                        parent: ColumnIndex(0),
                        child: ColumnIndex(1),
                        domain_id: ColumnIndex(0),
                        domain_filter: vec![comparison(Test::Equal)],
                        domain_view: Vec::new(),
                        output: Vec::new(),
                    },
                    scope: StepScope::PerFile,
                },
            ],
            Vec::new(),
        )],
    };
    let original = plans.clone();
    let extracted = from_plans(&plans);
    assert_eq!(
        plans, original,
        "metadata extraction does not alter executable source plans"
    );
    let mut output = Vec::new();
    for rule in extracted {
        output.push(Metadata::Rule {
            reads_project: rule.reads_project == ProjectRead::Reads,
            has_cross_steps: rule.has_cross_steps == CrossSteps::Present,
        });
        output.extend(
            rule.frontier_capture
                .iter()
                .copied()
                .enumerate()
                .map(|(input, retained)| Metadata::Capture {
                    input,
                    retained: retained == FrontierCapture::Captured,
                }),
        );
        for step in rule.steps {
            output.push(Metadata::Step {
                operation: step.operation,
                scope: step.scope,
            });
            output.extend(step.features.iter().map(|slot| Metadata::Feature {
                feature: slot.feature,
                source: slot.source,
            }));
        }
    }
    Ok(Cow::Owned(output))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
