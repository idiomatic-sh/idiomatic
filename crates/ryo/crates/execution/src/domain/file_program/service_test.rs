//! Pure request-graph identities and ordered dependency contracts.
use super::service::compile_prepared;
use super::types::{NodeInput, RequestGraph};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::RequestOperators;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::*;
use ryo_values::domain::plans::types::*;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Scenario {
    Origins,
    Bindings,
    Duplicates,
    Dependencies,
    InvalidSchema,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("caller_origins_do_not_split_pure_identity"),
        word: Word::Succeeds,
        input: || Scenario::Origins,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("different_bound_texts_keep_distinct_identity"),
        word: Word::Succeeds,
        input: || Scenario::Bindings,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("duplicate_steps_share_physical_identity"),
        word: Word::Succeeds,
        input: || Scenario::Duplicates,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordered_dependencies_remain_authored_inputs"),
        word: Word::Succeeds,
        input: || Scenario::Dependencies,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalid_schema_stays_deferred_in_same_graph"),
        word: Word::Succeeds,
        input: || Scenario::InvalidSchema,
        expect: Expect::Value(()),
    },
];
fn plan() -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: InputSource::Relation(Relation::Function),
                    kept: vec![ColumnIndex(0), ColumnIndex(2)],
                    comparisons: vec![Comparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                    }],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: InputSource::Step(StepId(0)),
                    columns: vec![ColumnIndex(1)],
                    distinct: RepeatedRows::Kept,
                },
            },
        ],
        emit: Emit {
            step: StepId(1),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}
use ryo_values::domain::plan_parts::types::Input as InputSource;
fn graph(
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
) -> Result<RequestGraph, OperatingError> {
    let operators = RequestOperators::prepare_with_schemas(
        plans,
        bindings,
        &[],
        &ryo_relations::domain::relations::service::kinds,
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    )?;
    Ok(compile_prepared(plans, &operators))
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    let first = plan();
    match scenario {
        Scenario::Origins => {
            let mut second = first.clone();
            if let Operation::Select { comparisons, .. } = &mut second.steps[0].operation {
                comparisons[0].origin = ryo_values::domain::plan_parts::types::ComparisonOrigin::Recorded(
            ryo_values::domain::plan_parts::types::ComparisonOriginId(7),
        );
            }
            let graph = graph(&[first, second], &[vec![], vec![]])?;
            let first = graph.program_nodes[0][0]
                .as_ref()
                .expect("first physical producer");
            let second = graph.program_nodes[1][0]
                .as_ref()
                .expect("second physical producer");
            assert_eq!(first, second);
        }
        Scenario::Bindings => {
            let mut bound = first;
            bound.parameters.push(PlanParameter {
                name: ParameterName("name".into()),
                kind: ParameterType::Text,
            });
            if let Operation::Select { comparisons, .. } = &mut bound.steps[0].operation {
                comparisons[0].right = Operand::Parameter(ParameterId(0));
            }
            let graph = graph(
                &[bound.clone(), bound],
                &[
                    vec![ParameterBinding::Text(LiteralText("main".into()))],
                    vec![ParameterBinding::Text(LiteralText("other".into()))],
                ],
            )?;
            let first = graph.program_nodes[0][0]
                .as_ref()
                .expect("first bound physical producer");
            let second = graph.program_nodes[1][0]
                .as_ref()
                .expect("second bound physical producer");
            assert_ne!(first, second);
        }
        Scenario::Duplicates => {
            let mut duplicate = first;
            duplicate.steps.insert(1, duplicate.steps[0].clone());
            duplicate.emit.step = StepId(2);
            duplicate.frontier = vec![InputSource::Step(StepId(1))];
            let graph = graph(&[duplicate.clone(), duplicate], &[vec![], vec![]])?;
            let node = graph.program_nodes[0][0].as_ref().unwrap();
            assert_eq!(graph.program_nodes[0][1].as_ref().unwrap(), node);
            assert_eq!(graph.program_nodes[1][0].as_ref().unwrap(), node);
            assert_eq!(graph.program_nodes[1][1].as_ref().unwrap(), node);
        }
        Scenario::Dependencies => {
            use crate::domain::operator::contract::types::DemandOrder;
            let mut union = first.clone();
            let mut other = union.steps[0].clone();
            if let Operation::Select { comparisons, .. } = &mut other.operation {
                comparisons[0].right =
                    Operand::Literal(QueryLiteral::Text(LiteralText("other".into())));
            }
            union.steps = vec![
                union.steps[0].clone(),
                other,
                PlanStep {
                    scope: StepScope::PerFile,
                    operation: Operation::Union {
                        left: InputSource::Step(StepId(0)),
                        right: InputSource::Step(StepId(1)),
                    },
                },
            ];
            union.emit.step = StepId(2);
            let mut joined = first.clone();
            joined.steps[1].operation = Operation::Join {
                left: InputSource::Step(StepId(0)),
                right: InputSource::Relation(Relation::Function),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(1),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            };
            let mut grouped = first;
            grouped.steps[1].operation = Operation::GroupCount {
                from: InputSource::Step(StepId(0)),
                keys: vec![ColumnIndex(0)],
                distinct: None,
            };
            let graph = graph(&[union, joined, grouped], &[vec![], vec![], vec![]])?;
            let union_left = *graph.program_nodes[0][0]
                .as_ref()
                .expect("first Union producer");
            let union_right = *graph.program_nodes[0][1]
                .as_ref()
                .expect("distinct Union producer");
            assert_ne!(
                union_left, union_right,
                "different predicates retain distinct producers"
            );
            let join_left = *graph.program_nodes[1][0].as_ref().expect("Join producer");
            let group_input = *graph.program_nodes[2][0]
                .as_ref()
                .expect("GroupCount producer");
            for (rule, step, expected, inputs) in [
                (
                    0,
                    2,
                    DemandOrder::LeftThenRight,
                    vec![NodeInput::Node(union_left), NodeInput::Node(union_right)],
                ),
                (
                    1,
                    1,
                    DemandOrder::LeftThenRight,
                    vec![
                        NodeInput::Node(join_left),
                        NodeInput::Relation(Relation::Function),
                    ],
                ),
                (2, 1, DemandOrder::Unary, vec![NodeInput::Node(group_input)]),
            ] {
                let id = graph.program_nodes[rule][step]
                    .as_ref()
                    .expect("operation physical root");
                let node = &graph.nodes[id.0];
                assert_eq!(node.contract.semantics.demand, expected);
                assert_eq!(
                    node.inputs, inputs,
                    "every authored edge retains its exact position"
                );
            }
        }
        Scenario::InvalidSchema => {
            let mut invalid = first;
            if let Operation::Project { columns, .. } = &mut invalid.steps[1].operation {
                columns[0] = ColumnIndex(999);
            }
            let graph = graph(&[invalid], &[vec![]])?;
            let id = graph.program_nodes[0][1].as_ref().unwrap();
            assert!(!graph.nodes[id.0].schema_valid);
        }
    }
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
