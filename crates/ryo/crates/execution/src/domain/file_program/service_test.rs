//! Pure request-graph identities and ordered dependency contracts.
use super::fixtures::{binding, bound_input, exporting, export, graph, observing, plan, rule};
use super::service::link::link;
use super::service::observations::{schedule, schemas};
use super::errors::FileProgramError;
use super::types::{NodeInput, ResolvedObservationBinding};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::observations::types::ObservationProducer;
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::selects;
use ryo_values::domain::plan_parts::types::*;
use ryo_values::domain::plans::types::*;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_bindings::types::{ParameterType, PlanParameter};

#[derive(Clone, Copy)]
enum Scenario {
    Origins,
    Bindings,
    Duplicates,
    Dependencies,
    InvalidContractShape,
    UnknownProducer,
    SelfRead,
    ExecutableNoQuery,
    RepeatedBinding,
    Cycle,
    NotCompiled,
    MissingOutput,
    IncompatibleColumns,
    DanglingStep,
}
type Input = Scenario;
type Value = ();
type Failure = FileProgramError;
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
        input: || Scenario::InvalidContractShape,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_unlisted_producer_is_unknown"),
        word: Word::Refuses,
        input: || Scenario::UnknownProducer,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, FileProgramError::UnknownProducer { .. }))
        }),
    },
    Case {
        name: CaseName("a_rule_cannot_read_its_own_export"),
        word: Word::Refuses,
        input: || Scenario::SelfRead,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, FileProgramError::SelfDependency { .. }))
        }),
    },
    Case {
        name: CaseName("a_listed_rule_cannot_be_declared_without_a_query"),
        word: Word::Refuses,
        input: || Scenario::ExecutableNoQuery,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, FileProgramError::ExecutableNoQuery { .. }))
        }),
    },
    Case {
        name: CaseName("a_name_is_bound_once_per_rule"),
        word: Word::Refuses,
        input: || Scenario::RepeatedBinding,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, FileProgramError::DuplicateBinding { .. }))
        }),
    },
    Case {
        name: CaseName("rules_that_read_each_other_have_no_order"),
        word: Word::Refuses,
        input: || Scenario::Cycle,
        expect: Expect::Error(FileProgramError::Cycle),
    },
    Case {
        name: CaseName("a_producer_without_a_plan_is_not_compiled"),
        word: Word::Refuses,
        input: || Scenario::NotCompiled,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, FileProgramError::NotCompiled { .. }))
        }),
    },
    Case {
        name: CaseName("a_plan_that_exports_no_such_name_misses_the_output"),
        word: Word::Refuses,
        input: || Scenario::MissingOutput,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, FileProgramError::MissingOutput { .. }))
        }),
    },
    Case {
        name: CaseName("an_export_of_other_columns_is_incompatible"),
        word: Word::Refuses,
        input: || Scenario::IncompatibleColumns,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                FileProgramError::IncompatibleObservationColumns { .. }
            ))
        }),
    },
    Case {
        name: CaseName("a_step_the_plan_does_not_have_is_an_invalid_reference"),
        word: Word::Refuses,
        input: || Scenario::DanglingStep,
        expect: Expect::Error(FileProgramError::InvalidReference),
    },
];
use ryo_values::domain::plan_parts::types::Input as InputSource;
fn subject(scenario: Input) -> Result<Value, Failure> {
    let first = plan();
    match scenario {
        Scenario::Origins => {
            let mut second = first.clone();
            if let Operation::Select { comparisons, .. } = &mut second.steps[0].operation {
                comparisons[0].origin =
                    ryo_values::domain::plan_parts::types::ComparisonOrigin::Recorded(
                        ryo_values::domain::plan_parts::types::ComparisonOriginId(7),
                    );
            }
            let graph = graph(&[first, second], &[vec![], vec![]]);
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
            );
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
            let graph = graph(&[duplicate.clone(), duplicate], &[vec![], vec![]]);
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
            let graph = graph(&[union, joined, grouped], &[vec![], vec![], vec![]]);
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
        Scenario::InvalidContractShape => {
            let mut invalid = first;
            if let Operation::Project { columns, .. } = &mut invalid.steps[1].operation {
                columns[0] = ColumnIndex(999);
            }
            let graph = graph(&[invalid], &[vec![]]);
            let id = graph.program_nodes[0][1].as_ref().unwrap();
            assert!(!graph.nodes[id.0].schema_valid);
        }
        Scenario::UnknownProducer => return schedule(&pair("absent")).map(|_| ()),
        Scenario::SelfRead => {
            return schedule(&[observing("only", vec![binding("rows", vec![export("only")])])])
                .map(|_| ());
        }
        Scenario::ExecutableNoQuery => {
            return schedule(&[observing(
                "only",
                vec![binding(
                    "rows",
                    vec![ObservationProducer::NoQuery {
                        rule: RuleId("only".into()),
                    }],
                )],
            )])
            .map(|_| ());
        }
        Scenario::RepeatedBinding => {
            return schedule(&[observing(
                "only",
                vec![binding("rows", vec![]), binding("rows", vec![])],
            )])
            .map(|_| ());
        }
        Scenario::Cycle => {
            return schedule(&[
                observing("one", vec![binding("rows", vec![export("two")])]),
                observing("two", vec![binding("rows", vec![export("one")])]),
            ])
            .map(|_| ());
        }
        Scenario::NotCompiled => {
            let rules = pair("first");
            return schemas::<Plan>(&rules[1], &rules, &[None, None]).map(|_| ());
        }
        Scenario::MissingOutput => {
            let rules = pair("first");
            return schemas(&rules[1], &rules, &[Some(plan()), None]).map(|_| ());
        }
        Scenario::IncompatibleColumns => {
            let rules = pair("first");
            return schemas(&rules[1], &rules, &[Some(exporting(vec![])), None]).map(|_| ());
        }
        Scenario::DanglingStep => {
            let mut dangling = plan();
            dangling.emit.step = StepId(99);
            let bound = ResolvedObservationBinding {
                input: ObservationBindingId(0),
                schema: bound_input("rows"),
                producers: vec![],
            };
            return link(vec![dangling], vec![vec![bound]]).map(|_| ());
        }
    }
    Ok(())
}

/// The two rules of a request in which `second` reads the export of `reads`.
fn pair(reads: &str) -> [RuleSpec; 2] {
    [
        rule("first"),
        observing("second", vec![binding("rows", vec![export(reads)])]),
    ]
}

/// Run every row that resolves.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// Run every row that is refused.
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
