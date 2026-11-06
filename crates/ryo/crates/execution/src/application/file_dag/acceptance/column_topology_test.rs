//! Request-wide physical replacement must respect every reader of canonical nodes.
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::{
        column::ColumnCandidates,
        service::{self, compile},
        types::{ExecutableProgram, PartitionRequirement, ProducerPartitions, Route},
    },
    file_program::types::{NodeId, RequestGraph},
    operating::errors::OperatingError,
    operator::contract::types::RequestOperators,
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    observations::types::{
        ObservationColumn, ObservationColumnName, ObservationName, PlanObservation,
    },
    plan_parts::types::{
        ColumnIndex, PlanComparison, HostedNumber, Input as PlanInput, JoinColumn, LiteralText,
        Operand, ParameterId, QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
    },
    plans::types::{Emit, Operation, Plan, PlanStep},
};

struct FileProducers;
impl ProducerPartitions for FileProducers {
    fn hosted(
        &self,
        _: RuleIndex,
        _: HostedNumber,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }

    fn parameter(
        &self,
        _: RuleIndex,
        _: ParameterId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::Unbound)
    }

    fn relation(&self, _: Relation) -> PartitionRequirement {
        PartitionRequirement::CurrentFile
    }

    fn observation(
        &self,
        _: RuleIndex,
        _: ryo_values::domain::plan_parts::types::ObservationBindingId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
}

fn column(at: u32) -> ColumnIndex {
    ColumnIndex(at)
}

/// A real selected-left/stored-right physical shape. The left frontier keeps
/// the source cut, while Join and Select are otherwise private ancestors.
fn plan(projected: Vec<ColumnIndex>) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Relation(Relation::Function),
                    columns: vec![column(0), column(2)],
                    distinct: RepeatedRows::Dropped,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Join {
                    left: PlanInput::Step(StepId(0)),
                    right: PlanInput::Relation(Relation::SemanticChild),
                    left_on: vec![column(0), column(1)],
                    right_on: vec![column(0), column(4)],
                    output: vec![
                        JoinColumn {
                            side: Side::Left,
                            column: column(0),
                        },
                        JoinColumn {
                            side: Side::Left,
                            column: column(1),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: column(1),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: column(3),
                        },
                    ],
                    filter: Vec::new(),
                    view: Vec::new(),
                    reach: None,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Step(StepId(1)),
                    comparisons: vec![PlanComparison {
                        test: Test::Equal,
                        left: Operand::Column(column(3)),
                        right: Operand::Literal(QueryLiteral::Text(LiteralText("body".into()))),
                        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                    }],
                    kept: Vec::new(),
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Step(StepId(2)),
                    columns: projected,
                    distinct: RepeatedRows::Dropped,
                },
            },
        ],
        comparison_origins: Vec::new(),
        emit: Emit {
            step: StepId(3),
            subject: column(0),
            key: vec![column(0)],
            headings: Vec::new(),
        },
        frontier: vec![PlanInput::Step(StepId(0))],
    }
}

fn join_output_observation() -> PlanObservation {
    PlanObservation {
        name: ObservationName("join-output".into()),
        source: PlanInput::Step(StepId(1)),
        columns: vec![
            ObservationColumn {
                name: ObservationColumnName("node".into()),
                kind: CellKind::Node,
            },
            ObservationColumn {
                name: ObservationColumnName("file".into()),
                kind: CellKind::File,
            },
            ObservationColumn {
                name: ObservationColumnName("child".into()),
                kind: CellKind::Node,
            },
            ObservationColumn {
                name: ObservationColumnName("field".into()),
                kind: CellKind::Symbol,
            },
        ],
        scope: StepScope::PerFile,
    }
}

fn ordinary_plan() -> Plan {
    plan(vec![column(0), column(1), column(2)])
}

fn graph(plans: &[Plan]) -> (RequestOperators, RequestGraph) {
    let bindings = vec![Vec::new(); plans.len()];
    let operators = RequestOperators::prepare(
        plans,
        &bindings,
        &[],
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    )
    .unwrap();
    let graph = crate::domain::file_program::service::compile_prepared(plans, &operators);
    (operators, graph)
}

fn node(graph: &RequestGraph, rule: usize, step: usize) -> NodeId {
    *graph.program_nodes[rule][step].as_ref().unwrap()
}

fn program(plans: &[Plan], operators: &RequestOperators, graph: RequestGraph) -> ExecutableProgram {
    compile(plans, graph, &FileProducers, operators).unwrap()
}

/// An instruction has exactly one route: the accessors agree with the kind, and at most one is set.
fn assert_one_route(program: &ExecutableProgram) {
    for region in program.regions() {
        let route = &region.route;
        let named = [
            service::ordered_existence(route).is_some(),
            service::column(route).is_some(),
            service::selection(route).is_some(),
            service::native_positions(route).is_some(),
        ];
        assert!(named.iter().filter(|each| **each).count() <= 1);
        assert_eq!(
            named.iter().any(|each| *each),
            !matches!(route, Route::Generic)
        );
    }
}

fn column_count(program: &ExecutableProgram) -> usize {
    assert_one_route(program);
    program
        .regions()
        .iter()
        .filter(|region| service::column(&region.route).is_some())
        .count()
}

fn exclusive_chain_lowers_once_and_duplicate_rule_shares_one_physical_owner() -> usize {
    let single = vec![ordinary_plan()];
    let (operators, request) = graph(&single);
    assert_eq!(ColumnCandidates::capture(&request).fences().count(), 4);
    let compiled = program(&single, &operators, request);
    assert_eq!(column_count(&compiled), 1);
    assert!(
        service::column(&compiled.regions()[compiled.rules()[0].emit.as_ref().unwrap().0].route)
            .is_some()
    );

    let duplicate = vec![ordinary_plan(), ordinary_plan()];
    let (operators, request) = graph(&duplicate);
    for step in 0..4 {
        assert_eq!(node(&request, 0, step), node(&request, 1, step));
    }
    let left_node = node(&request, 0, 0);
    assert_eq!(ColumnCandidates::capture(&request).fences().count(), 8);
    let (ordinary_operators, mut without_replacement) = graph(&duplicate);
    let shared_join = node(&without_replacement, 0, 1);
    without_replacement.derived_sources[shared_join.0] = true;
    let ordinary = program(&duplicate, &ordinary_operators, without_replacement);
    let compiled = program(&duplicate, &operators, request);
    assert_eq!(column_count(&compiled), 1);
    assert_eq!(compiled.rules()[0].emit, compiled.rules()[1].emit);
    let left = compiled.logical_regions()[left_node.0];
    let ordinary_left = ordinary.logical_regions()[left_node.0];
    assert_eq!(
        compiled.regions()[left.0].consumers,
        ordinary.regions()[ordinary_left.0].consumers + 1,
        "one physical alternative owns one extra source reader, never one per duplicate rule"
    );
    column_count(&compiled)
}

fn distinct_targets_with_shared_join_and_filter_keep_both_ordinary_in_either_rule_order() -> usize {
    let first = ordinary_plan();
    let second = plan(vec![column(0), column(1)]);
    for plans in [
        vec![first.clone(), second.clone()],
        vec![second.clone(), first.clone()],
    ] {
        let (operators, request) = graph(&plans);
        assert_eq!(node(&request, 0, 1), node(&request, 1, 1));
        assert_eq!(node(&request, 0, 2), node(&request, 1, 2));
        assert_ne!(node(&request, 0, 3), node(&request, 1, 3));
        let filter_node = node(&request, 0, 2);
        assert_eq!(ColumnCandidates::capture(&request).fences().count(), 8);
        let compiled = program(&plans, &operators, request);
        assert_eq!(column_count(&compiled), 0);
        assert_ne!(compiled.rules()[0].emit, compiled.rules()[1].emit);
        let filter = compiled.logical_regions()[filter_node.0];
        assert_eq!(compiled.regions()[filter.0].consumers, 2);
    }
    0
}

fn emit_frontier_named_and_derived_ancestor_boundaries_prevent_replacement() -> usize {
    let mut emit = ordinary_plan();
    emit.emit.step = StepId(1);
    emit.frontier.push(PlanInput::Step(StepId(3)));

    let mut frontier = ordinary_plan();
    frontier.frontier.push(PlanInput::Step(StepId(2)));

    let mut named = ordinary_plan();
    named.observations.push(join_output_observation());

    for (observed, boundary_step) in [(emit, 1), (frontier, 2), (named, 1)] {
        let plans = vec![observed];
        let (operators, request) = graph(&plans);
        let boundary = node(&request, 0, boundary_step);
        let terminal = node(&request, 0, 3);
        let compiled = program(&plans, &operators, request);
        assert_eq!(column_count(&compiled), 0);
        assert_ne!(
            compiled.logical_regions()[boundary.0],
            compiled.logical_regions()[terminal.0],
            "an externally observed ancestor keeps its own physical publication"
        );
    }

    let plans = vec![ordinary_plan()];
    let (operators, mut request) = graph(&plans);
    let filter = node(&request, 0, 2);
    assert_eq!(ColumnCandidates::capture(&request).fences().count(), 4);
    request.derived_sources[filter.0] = true;
    assert_eq!(column_count(&program(&plans, &operators, request)), 0);
    0
}

/// A native-eligible Join that a named observation publishes and a Project reads is read by
/// rows, so neither reader may be handed a native positions publication.
fn native_join_read_by_rows_keeps_no_native_positions() -> usize {
    let read = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::SyntaxTree),
                    comparisons: Vec::new(),
                    kept: Vec::new(),
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Join {
                    left: PlanInput::Step(StepId(0)),
                    right: PlanInput::Relation(Relation::SyntaxChild),
                    left_on: vec![column(0)],
                    right_on: vec![column(0)],
                    output: vec![
                        JoinColumn {
                            side: Side::Left,
                            column: column(0),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: column(1),
                        },
                    ],
                    filter: Vec::new(),
                    view: Vec::new(),
                    reach: None,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Step(StepId(1)),
                    columns: vec![column(0), column(1)],
                    distinct: RepeatedRows::Dropped,
                },
            },
        ],
        comparison_origins: Vec::new(),
        emit: Emit {
            step: StepId(2),
            subject: column(0),
            key: vec![column(0)],
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    };
    let mut observed = read.clone();
    observed.observations.push(PlanObservation {
        name: ObservationName("parent-child".into()),
        source: PlanInput::Step(StepId(1)),
        columns: ["parent", "child"]
            .map(|name| ObservationColumn {
                name: ObservationColumnName(name.into()),
                kind: CellKind::Node,
            })
            .to_vec(),
        scope: StepScope::PerFile,
    });
    let native = |plan: &Plan| {
        let plans = vec![plan.clone()];
        let (operators, request) = graph(&plans);
        let join = node(&request, 0, 1);
        let compiled = program(&plans, &operators, request);
        assert_one_route(&compiled);
        service::native_positions(&compiled.regions()[compiled.logical_regions()[join.0].0].route)
            .is_some()
    };
    assert!(
        native(&read),
        "the Join is native-eligible when nothing reads its rows"
    );
    assert!(
        !native(&observed),
        "an observed Join is read by rows and keeps no native positions"
    );
    0
}

#[derive(Clone, Copy)]
enum Scenario {
    ExclusiveAndDuplicate,
    SharedAncestors,
    ObservedAncestors,
    NativeJoinObserved,
}

type Input = Scenario;
type Value = usize;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("exclusive_chain_is_selected_once_even_for_duplicate_rules"),
        word: Word::Succeeds,
        input: || Scenario::ExclusiveAndDuplicate,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("shared_join_and_filter_keep_both_targets_ordinary_in_either_order"),
        word: Word::Succeeds,
        input: || Scenario::SharedAncestors,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("emit_frontier_named_and_derived_roots_prevent_replacement"),
        word: Word::Succeeds,
        input: || Scenario::ObservedAncestors,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("native_join_observed_by_a_named_output_keeps_no_native_positions"),
        word: Word::Succeeds,
        input: || Scenario::NativeJoinObserved,
        expect: Expect::Value(0),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::ExclusiveAndDuplicate => {
            exclusive_chain_lowers_once_and_duplicate_rule_shares_one_physical_owner()
        }
        Scenario::SharedAncestors => {
            distinct_targets_with_shared_join_and_filter_keep_both_ordinary_in_either_rule_order()
        }
        Scenario::ObservedAncestors => {
            emit_frontier_named_and_derived_ancestor_boundaries_prevent_replacement()
        }
        Scenario::NativeJoinObserved => native_join_read_by_rows_keeps_no_native_positions(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
