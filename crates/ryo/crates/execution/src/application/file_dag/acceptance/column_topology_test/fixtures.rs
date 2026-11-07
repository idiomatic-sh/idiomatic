//! Authored plans and request graphs for the column topology controls.
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::service::compile;
use crate::domain::executable_program::service;
use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::executable_program::types::PartitionRequirement;
use crate::domain::executable_program::types::ProducerPartitions;
use crate::domain::executable_program::types::Route;
use crate::domain::file_program::types::NodeId;
use crate::domain::file_program::types::RequestGraph;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::RequestOperators;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::observations::types::ObservationColumn;
use ryo_values::domain::observations::types::ObservationColumnName;
use ryo_values::domain::observations::types::ObservationName;
use ryo_values::domain::observations::types::PlanObservation;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::ParameterId;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::plans::types::PlanStep;

pub(super) struct FileProducers;
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

pub(super) fn column(at: u32) -> ColumnIndex {
    ColumnIndex(at)
}

/// A real selected-left/stored-right physical shape. The left frontier keeps
/// the source cut, while Join and Select are otherwise private ancestors.
pub(super) fn plan(projected: Vec<ColumnIndex>) -> Plan {
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

pub(super) fn join_output_observation() -> PlanObservation {
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

pub(super) fn ordinary_plan() -> Plan {
    plan(vec![column(0), column(1), column(2)])
}

pub(super) fn graph(plans: &[Plan]) -> (RequestOperators, RequestGraph) {
    let bindings = vec![Vec::new(); plans.len()];
    let operators = crate::application::file_dag::service::prepare(
        plans,
        &bindings,
        &[],
        (std::sync::Arc::new(
            crate::application::file_dag::fixtures::derived(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    )
    .unwrap();
    let graph = crate::domain::file_program::service::compile_prepared(plans, &operators);
    (operators, graph)
}

pub(super) fn node(graph: &RequestGraph, rule: usize, step: usize) -> NodeId {
    *graph.program_nodes[rule][step].as_ref().unwrap()
}

pub(super) fn program(plans: &[Plan], operators: &RequestOperators, graph: RequestGraph) -> ExecutableProgram {
    compile(plans, graph, &FileProducers, operators, crate::domain::executable_program::types::RoutePolicy::default()).unwrap()
}

/// An instruction has exactly one route: the accessors agree with the kind, and at most one is set.
pub(super) fn assert_one_route(program: &ExecutableProgram) {
    for region in program.regions() {
        let route = &region.route();
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

pub(super) fn column_count(program: &ExecutableProgram) -> usize {
    assert_one_route(program);
    program
        .regions()
        .iter()
        .filter(|region| service::column(&region.route()).is_some())
        .count()
}
