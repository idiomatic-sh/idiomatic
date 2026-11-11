//! Schema, publication and refusal controls through production request preparation.
#[cfg(test)]
use super::fixtures::{native_plan, plan, prepared, select, step};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::file_program::service::compile_prepared_observations;
use ryo_execution_domain::domain::file_program::types::NodeInput;
use ryo_execution_domain::domain::observation_schedule::types::ObservationRequest;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as CellValue;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;

fn each_native_step_answers_its_output_kinds() {
    let plans = [native_plan(false)];
    let operators = crate::application::file_dag::service::prepare(
        &plans,
        &[vec![]],
        &[],
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
    )
    .unwrap();
    assert_eq!(
        operators.rules[0]
            .iter()
            .map(|contract| contract.output_kinds.clone().unwrap())
            .collect::<Vec<_>>(),
        [
            vec![
                CellKind::Node,
                CellKind::Symbol,
                CellKind::Symbol,
                CellKind::File,
                CellKind::Span
            ],
            vec![CellKind::Node]
        ]
    );
}

fn the_step_handed_to_cross_file_steps_is_its_frontier() {
    let plans = [native_plan(true)];
    let operators = crate::application::file_dag::service::prepare(
        &plans,
        &[vec![]],
        &[],
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
    )
    .unwrap();
    assert_eq!(
        operators.rules[0][1].schema,
        Ok(vec![CellKind::Symbol, CellKind::Number])
    );
    let request = prepared(plans[0].clone()).unwrap();
    let program = request.program();
    assert!(matches!(
        program.rules()[0].frontier[0],
        Ok(ryo_execution_domain::domain::executable_program::types::InputSource::Region(_))
    ));
}

fn a_step_dependency_and_its_terminal_publication_are_counted_once_each() {
    let request = prepared(native_plan(false)).unwrap();
    let program = request.program();
    let origins = &program.rules()[0].original_steps;
    let first = origins[0].as_ref().unwrap().logical;
    let last = origins[1].as_ref().unwrap().logical;
    let first_region = program.logical_regions()[first.0];
    let last_region = program.logical_regions()[last.0];
    assert_ne!(
        first_region, last_region,
        "distinct projection owns a real input publication"
    );
    assert_eq!(program.regions()[first_region.0].consumers().0, 1);
    assert_eq!(program.regions()[last_region.0].consumers().0, 1);
}

fn metadata_preparation_propagates_select_extend_and_graph_schemas_without_rows() {
    let plans = [plan(
        vec![
            step(select(PlanInput::Relation(Relation::Function))),
            step(Operation::Extend {
                from: PlanInput::Step(StepId(0)),
                values: vec![Operand::Literal(QueryLiteral::Text(LiteralText(
                    "do-not-intern-layout".into(),
                )))],
            }),
            step(Operation::Closure {
                edges: PlanInput::Relation(Relation::Function),
                keys: vec![],
                source: ColumnIndex(0),
                destination: ColumnIndex(0),
            }),
        ],
        vec![],
    )];
    let operators = crate::application::file_dag::service::prepare_with_schemas(
        &plans,
        &[vec![]],
        &[],
        &|_| &[CellKind::Symbol, CellKind::File],
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
    )
    .unwrap();
    let graph = compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
    assert_eq!(operators.preparations.0, 3);
    let schemas = graph.program_nodes[0]
        .iter()
        .map(|node| {
            graph.nodes[node.as_ref().unwrap().0]
                .contract
                .output_kinds
                .clone()
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        schemas,
        [
            vec![CellKind::Symbol, CellKind::File],
            vec![CellKind::Symbol, CellKind::File, CellKind::Symbol],
            vec![CellKind::Symbol; 2]
        ]
    );
}

fn preparation_keeps_ordered_relation_dependencies_without_probing_availability() {
    let plans = [plan(
        vec![step(Operation::Join {
            left: PlanInput::Relation(Relation::Function),
            right: PlanInput::Relation(Relation::Comment),
            left_on: vec![],
            right_on: vec![],
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            }],
            filter: vec![],
            view: vec![],
            reach: None,
        })],
        vec![],
    )];
    let probes = std::cell::RefCell::new(Vec::new());
    let metadata = |relation| {
        probes.borrow_mut().push(relation);
        &[CellKind::Symbol, CellKind::File][..]
    };
    let operators = crate::application::file_dag::service::prepare_with_schemas(
        &plans,
        &[vec![]],
        &[],
        &metadata,
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
    )
    .unwrap();
    let graph = compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
    assert_eq!(
        &probes.borrow()[..2],
        [Relation::Function, Relation::Comment]
    );
    assert_eq!(
        graph.nodes[0].inputs,
        [
            NodeInput::Relation(Relation::Function),
            NodeInput::Relation(Relation::Comment)
        ]
    );
    // Actual missing-provider refusal is exercised by the facade runtime service_test's missing-demanded-right-source case.
}

fn empty_native_frontier_has_the_deep_walk_schema_before_any_file_is_read() {
    let plans = [plan(
        vec![step(Operation::Axis {
            from: PlanInput::Relation(Relation::SyntaxTree),
            edges: PlanInput::Relation(Relation::SyntaxChild),
            domain: PlanInput::Relation(Relation::SyntaxTree),
            direction: Direction::Deep,
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            domain_filter: vec![],
            domain_view: vec![],
            output: vec![],
        })],
        vec![PlanInput::Step(StepId(0))],
    )];
    let operators = crate::application::file_dag::service::prepare(
        &plans,
        &[vec![]],
        &[],
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
    )
    .unwrap();
    let graph = compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
    let output = graph.program_nodes[0][0].as_ref().unwrap();
    assert_eq!(
        graph.nodes[output.0].contract.output_kinds,
        Ok(vec![
            CellKind::Node,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::File,
            CellKind::Span,
            CellKind::Node
        ])
    );
}

fn preparing_from_borrowed_schemas_preserves_cell_allocations_and_prepared_metadata() {
    let mut cells = Vec::with_capacity(97);
    cells.push(CellValue::Number(Integer(7)));
    let computed = Table {
        kinds: vec![CellKind::Number],
        cells,
        count: RowCount(1),
    };
    let mut native_cells = Vec::with_capacity(79);
    native_cells.push(CellValue::Number(Integer(17)));
    // Payload bytes are deliberately unrelated to the borrowed schema: preparation cannot inspect them.
    let native = Table {
        kinds: kinds(Relation::SyntaxTree).to_vec(),
        cells: native_cells,
        count: RowCount(1),
    };
    let before = (
        computed.cells.as_ptr(),
        computed.cells.capacity(),
        native.cells.as_ptr(),
        native.cells.capacity(),
    );
    let plans = [plan(
        vec![
            step(select(PlanInput::Relation(Relation::Function))),
            step(select(PlanInput::Relation(Relation::SyntaxTree))),
            step(select(PlanInput::Relation(Relation::SyntaxChild))),
        ],
        vec![],
    )];
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the test overrides two relations of the catalog and reads every other as the catalog declares it"
    )]
    let metadata = |relation| match relation {
        Relation::Function => computed.kinds.as_slice(),
        Relation::SyntaxTree => native.kinds.as_slice(),
        _ => kinds(relation),
    };
    let operators = crate::application::file_dag::service::prepare_with_schemas(
        &plans,
        &[vec![]],
        &[],
        &metadata,
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
    )
    .unwrap();
    let prepared_kinds = operators.rules[0][1]
        .output_kinds
        .as_ref()
        .unwrap()
        .as_ptr();
    let first = compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
    let again = compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
    assert_eq!(computed.cells, [CellValue::Number(Integer(7))]);
    assert_eq!(native.cells, [CellValue::Number(Integer(17))]);
    assert_eq!(
        before,
        (
            computed.cells.as_ptr(),
            computed.cells.capacity(),
            native.cells.as_ptr(),
            native.cells.capacity()
        )
    );
    assert_eq!(
        operators.rules[0][1]
            .output_kinds
            .as_ref()
            .unwrap()
            .as_ptr(),
        prepared_kinds
    );
    for graph in [first, again] {
        let output = graph.program_nodes[0][2].as_ref().unwrap();
        assert_eq!(
            graph.nodes[output.0].contract.output_kinds,
            Ok(kinds(Relation::SyntaxChild).to_vec())
        );
    }
}

fn an_invalid_frontier_keeps_the_prior_source_and_its_own_typed_refusal() {
    let request = prepared(plan(
        vec![],
        vec![
            PlanInput::Relation(Relation::Function),
            PlanInput::Step(StepId(99)),
        ],
    ))
    .unwrap();
    let frontier = &request.program().rules()[0].frontier;
    assert!(matches!(
        frontier[0],
        Ok(
            ryo_execution_domain::domain::executable_program::types::InputSource::Relation(
                Relation::Function
            )
        )
    ));
    assert_eq!(frontier[1], Err(OperatingError::NotRun));
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    EachNativeStepAnswersItsOutputKinds,
    TheStepHandedToCrossFileStepsIsItsFrontier,
    AStepDependencyAndItsTerminalPublicationAreCountedOnceEach,
    MetadataPreparationPropagatesSelectExtendAndGraphSchemasWithoutRows,
    PreparationKeepsOrderedRelationDependenciesWithoutProbingAvailability,
    EmptyNativeFrontierHasTheDeepWalkSchemaBeforeAnyFileIsRead,
    PreparingFromBorrowedSchemasPreservesCellAllocationsAndPreparedMetadata,
    AnInvalidFrontierKeepsThePriorSourceAndItsOwnTypedRefusal,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_native_step_answers_its_output_kinds"),
        word: Word::Succeeds,
        input: || Scenario::EachNativeStepAnswersItsOutputKinds,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the_step_handed_to_cross_file_steps_is_its_frontier"),
        word: Word::Succeeds,
        input: || Scenario::TheStepHandedToCrossFileStepsIsItsFrontier,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_step_dependency_and_its_terminal_publication_are_counted_once_each"),
        word: Word::Succeeds,
        input: || Scenario::AStepDependencyAndItsTerminalPublicationAreCountedOnceEach,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "metadata_preparation_propagates_select_extend_and_graph_schemas_without_rows",
        ),
        word: Word::Succeeds,
        input: || Scenario::MetadataPreparationPropagatesSelectExtendAndGraphSchemasWithoutRows,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "preparation_keeps_ordered_relation_dependencies_without_probing_availability",
        ),
        word: Word::Succeeds,
        input: || Scenario::PreparationKeepsOrderedRelationDependenciesWithoutProbingAvailability,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("empty_native_frontier_has_the_deep_walk_schema_before_any_file_is_read"),
        word: Word::Succeeds,
        input: || Scenario::EmptyNativeFrontierHasTheDeepWalkSchemaBeforeAnyFileIsRead,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "preparing_from_borrowed_schemas_preserves_cell_allocations_and_prepared_metadata",
        ),
        word: Word::Succeeds,
        input: || Scenario::PreparingFromBorrowedSchemasPreservesCellAllocationsAndPreparedMetadata,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_invalid_frontier_keeps_the_prior_source_and_its_own_typed_refusal"),
        word: Word::Succeeds,
        input: || Scenario::AnInvalidFrontierKeepsThePriorSourceAndItsOwnTypedRefusal,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::EachNativeStepAnswersItsOutputKinds => {
            each_native_step_answers_its_output_kinds();
            Ok(())
        }
        Scenario::TheStepHandedToCrossFileStepsIsItsFrontier => {
            the_step_handed_to_cross_file_steps_is_its_frontier();
            Ok(())
        }
        Scenario::AStepDependencyAndItsTerminalPublicationAreCountedOnceEach => {
            a_step_dependency_and_its_terminal_publication_are_counted_once_each();
            Ok(())
        }
        Scenario::MetadataPreparationPropagatesSelectExtendAndGraphSchemasWithoutRows => {
            metadata_preparation_propagates_select_extend_and_graph_schemas_without_rows();
            Ok(())
        }
        Scenario::PreparationKeepsOrderedRelationDependenciesWithoutProbingAvailability => {
            preparation_keeps_ordered_relation_dependencies_without_probing_availability();
            Ok(())
        }
        Scenario::EmptyNativeFrontierHasTheDeepWalkSchemaBeforeAnyFileIsRead => {
            empty_native_frontier_has_the_deep_walk_schema_before_any_file_is_read();
            Ok(())
        }
        Scenario::PreparingFromBorrowedSchemasPreservesCellAllocationsAndPreparedMetadata => {
            preparing_from_borrowed_schemas_preserves_cell_allocations_and_prepared_metadata();
            Ok(())
        }
        Scenario::AnInvalidFrontierKeepsThePriorSourceAndItsOwnTypedRefusal => {
            an_invalid_frontier_keeps_the_prior_source_and_its_own_typed_refusal();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
