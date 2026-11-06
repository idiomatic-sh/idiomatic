//! Schema, publication and refusal controls through production request preparation.

use crate::domain::{
    file_program::{
        service::compile_prepared_observations,
        types::{NodeInput, ObservationRequest, RuleSchedule},
    },
    operating::errors::OperatingError,
    operator::contract::types::RequestOperators,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, Integer, Relation, Value},
};
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, Direction, Input, JoinColumn, LiteralText, Operand, QueryLiteral,
        RepeatedRows, Side, StepId,
    },
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
    program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rows::types::{RowCount, Table},
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::domain::completion::types::CoverageState;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::table_words::types::DecodeMode;
use std::{collections::BTreeMap, sync::Arc};

// Fixtures assemble authored requests only. The production compiler derives every schema,
// retention boundary, reader count and frontier reference asserted below.
fn plan(steps: Vec<PlanStep>, frontier: Vec<Input>) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        emit: Emit {
            step: StepId(steps.len().saturating_sub(1) as u32),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
        frontier,
        comparison_origins: vec![],
    }
}
fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}
fn select(from: Input) -> Operation {
    Operation::Select {
        from,
        comparisons: vec![],
        kept: vec![],
    }
}
fn native_plan(cross: bool) -> Plan {
    let mut tail = step(if cross {
        Operation::GroupCount {
            from: Input::Step(StepId(0)),
            keys: vec![ColumnIndex(1)],
            distinct: None,
        }
    } else {
        Operation::Project {
            from: Input::Step(StepId(0)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        }
    });
    if cross {
        tail.scope = StepScope::CrossFile;
    }
    plan(
        vec![step(select(Input::Relation(Relation::SyntaxTree))), tail],
        if cross {
            vec![Input::Step(StepId(0))]
        } else {
            vec![]
        },
    )
}
fn prepared(
    plan: Plan,
) -> Result<
    Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    OperatingError,
> {
    let rule = RuleSpec {
        id: RuleId("prepared-layout".into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId("prepared-layout".into()),
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
    };
    crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
            bindings: vec![vec![]],
        },
        &[rule],
        &[Digest([0; 16])],
        &[],
        crate::application::file_dag::testing::program_derived_port(),
        ObservationIdentity::Independent,
    )
}

#[test]
fn only_the_emitted_native_step_is_a_retention_root_when_each_is_read_once() {
    let plans = [native_plan(false)];
    let operators = RequestOperators::prepare(
        &plans,
        &[vec![]],
        &[],
        crate::application::file_dag::testing::program_derived_port(),
    )
    .unwrap();
    assert_eq!(
        crate::domain::file_program::service::materialized_steps(&plans[0]),
        [false, true]
    );
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

#[test]
fn the_step_handed_to_cross_file_steps_is_a_retention_root() {
    let plans = [native_plan(true)];
    let operators = RequestOperators::prepare(
        &plans,
        &[vec![]],
        &[],
        crate::application::file_dag::testing::program_derived_port(),
    )
    .unwrap();
    assert_eq!(
        crate::domain::file_program::service::materialized_steps(&plans[0]),
        [true, true]
    );
    assert_eq!(
        operators.rules[0][1].schema,
        Ok(vec![CellKind::Symbol, CellKind::Number])
    );
    let request = prepared(plans[0].clone()).unwrap();
    let program = request.program();
    assert!(matches!(
        program.rules()[0].frontier[0],
        Ok(crate::domain::executable_program::types::InputSource::Region(_))
    ));
}

#[test]
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
    assert_eq!(program.regions()[first_region.0].consumers, 1);
    assert_eq!(program.regions()[last_region.0].consumers, 1);
}

#[test]
fn metadata_preparation_propagates_select_extend_and_graph_schemas_without_rows() {
    let plans = [plan(
        vec![
            step(select(Input::Relation(Relation::Function))),
            step(Operation::Extend {
                from: Input::Step(StepId(0)),
                values: vec![Operand::Literal(QueryLiteral::Text(LiteralText(
                    "do-not-intern-layout".into(),
                )))],
            }),
            step(Operation::Closure {
                edges: Input::Relation(Relation::Function),
                keys: vec![],
                source: ColumnIndex(0),
                destination: ColumnIndex(0),
            }),
        ],
        vec![],
    )];
    let operators = RequestOperators::prepare_with_schemas(
        &plans,
        &[vec![]],
        &[],
        &|_| &[CellKind::Symbol, CellKind::File],
        crate::application::file_dag::testing::program_derived_port(),
    )
    .unwrap();
    let graph = compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
    assert_eq!(operators.preparations, 3);
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

#[test]
fn preparation_keeps_ordered_relation_dependencies_without_probing_availability() {
    let plans = [plan(
        vec![step(Operation::Join {
            left: Input::Relation(Relation::Function),
            right: Input::Relation(Relation::Comment),
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
    let operators = RequestOperators::prepare_with_schemas(
        &plans,
        &[vec![]],
        &[],
        &metadata,
        crate::application::file_dag::testing::program_derived_port(),
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
    // Actual missing-provider refusal is exercised by compiled_parity's missing-demanded-right-source case.
}

#[test]
fn empty_native_frontier_has_the_deep_walk_schema_before_any_file_is_read() {
    let plans = [plan(
        vec![step(Operation::Axis {
            from: Input::Relation(Relation::SyntaxTree),
            edges: Input::Relation(Relation::SyntaxChild),
            domain: Input::Relation(Relation::SyntaxTree),
            direction: Direction::Deep,
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            domain_filter: vec![],
            domain_view: vec![],
            output: vec![],
        })],
        vec![Input::Step(StepId(0))],
    )];
    let operators = RequestOperators::prepare(
        &plans,
        &[vec![]],
        &[],
        crate::application::file_dag::testing::program_derived_port(),
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

#[test]
fn preparing_from_borrowed_schemas_preserves_cell_allocations_and_prepared_metadata() {
    let mut cells = Vec::with_capacity(97);
    cells.push(Value::Number(Integer(7)));
    let computed = Table {
        kinds: vec![CellKind::Number],
        cells,
        count: RowCount(1),
    };
    let mut native_cells = Vec::with_capacity(79);
    native_cells.push(Value::Number(Integer(17)));
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
            step(select(Input::Relation(Relation::Function))),
            step(select(Input::Relation(Relation::SyntaxTree))),
            step(select(Input::Relation(Relation::SyntaxChild))),
        ],
        vec![],
    )];
    let metadata = |relation| match relation {
        Relation::Function => computed.kinds.as_slice(),
        Relation::SyntaxTree => native.kinds.as_slice(),
        _ => kinds(relation),
    };
    let operators = RequestOperators::prepare_with_schemas(
        &plans,
        &[vec![]],
        &[],
        &metadata,
        crate::application::file_dag::testing::program_derived_port(),
    )
    .unwrap();
    let prepared_kinds = operators.rules[0][1]
        .output_kinds
        .as_ref()
        .unwrap()
        .as_ptr();
    let first = compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
    let again = compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
    assert_eq!(computed.cells, [Value::Number(Integer(7))]);
    assert_eq!(native.cells, [Value::Number(Integer(17))]);
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

#[test]
fn an_invalid_frontier_keeps_the_prior_source_and_its_own_typed_refusal() {
    let request = prepared(plan(
        vec![],
        vec![Input::Relation(Relation::Function), Input::Step(StepId(99))],
    ))
    .unwrap();
    let frontier = &request.program().rules()[0].frontier;
    assert!(matches!(
        frontier[0],
        Ok(crate::domain::executable_program::types::InputSource::Relation(Relation::Function))
    ));
    assert_eq!(frontier[1], Err(OperatingError::NotRun));
}

// Preparation leaves a stored frontier packed. Its decode runs on the first demand (open of the
// project run), and a decode failure is the same typed refusal preparation used to record: it is
// consulted at the same place, the first supply of that source, so the error precedence holds.

/// A varint whose continuation bit is never closed: no canonical frame spells it.
fn cut_words() -> Arc<ryo_values::domain::stored_facts::types::KeptWords> {
    Arc::new(ryo_values::domain::stored_facts::types::KeptWords(
        ryo_values::domain::stored_facts::types::PackedWords(vec![0x80]),
    ))
}
fn packed(input: Input, mode: DecodeMode) -> crate::application::file_dag::frontiers::types::PackedFrontier {
    crate::application::file_dag::frontiers::types::PackedFrontier {
        input,
        words: cut_words(),
        mode,
        coverage: CoverageState::Complete,
    }
}

#[test]
fn a_cut_frontier_is_refused_by_its_input_when_admitted_in_complete_mode() {
    let input = Input::Relation(Relation::Function);
    let refused = crate::application::file_dag::frontiers::service::admit(packed(input, DecodeMode::Complete), &mut Symbols::default())
        .err()
        .expect("a cut frame is never complete");
    assert_eq!(refused.input, input);
    assert!(matches!(
        refused.error,
        OperatingError::ObservationIncomplete { .. }
    ));
}

#[test]
fn a_cut_step_frontier_is_refused_by_its_input_when_admitted_in_complete_mode() {
    let input = Input::Step(StepId(0));
    let refused = crate::application::file_dag::frontiers::service::admit(packed(input, DecodeMode::Complete), &mut Symbols::default())
        .err()
        .expect("a cut frame is never complete");
    assert_eq!(refused.input, input);
    assert!(matches!(
        refused.error,
        OperatingError::ObservationIncomplete { .. }
    ));
}

#[test]
fn the_permissive_diagnostic_decode_never_refuses_a_packed_frontier() {
    let input = Input::Relation(Relation::Function);
    let admitted = crate::application::file_dag::frontiers::service::admit(packed(input, DecodeMode::Permissive), &mut Symbols::default())
        .ok()
        .expect("diagnostic frontiers decode what they can");
    assert!(matches!(admitted, FrontierInput::Decoded { .. }));
}
