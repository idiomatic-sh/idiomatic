//! Selected RHS projection and duplicate order through the real compiled Join.
use super::*;
use crate::domain::table_model::service::take_read_row_counts;
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn, NodeId},
};

fn request() -> Plan {
    let select = |relation| PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: PlanInput::Relation(relation),
            comparisons: vec![],
            kept: vec![ColumnIndex(1), ColumnIndex(0)],
        },
    };
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        steps: vec![
            select(Relation::SemanticChild),
            select(Relation::SyntaxChild),
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Join {
                    left: PlanInput::Step(StepId(0)),
                    right: PlanInput::Step(StepId(1)),
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                    output: vec![
                        JoinColumn {
                            side: Side::Left,
                            column: ColumnIndex(1),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(1),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(1),
                        },
                    ],
                    filter: vec![],
                    view: vec![],
                    reach: None,
                },
            },
        ],
        emit: Emit {
            step: StepId(2),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    }
}
// Canonical five-column provider rows. The selected RHS retains reordered columns
// over the original population; duplicate values are real rows, not installed positions.
fn edges(pairs: &[(u64, u64)]) -> Vec<FactColumn> {
    let mut columns = vec![FactColumn(vec![]); 5];
    for (ordinal, &(parent, child)) in pairs.iter().enumerate() {
        for (column, value) in columns.iter_mut().zip([
            Cell::Node(NodeId(parent)),
            Cell::Node(NodeId(child)),
            Cell::Number(Integer(ordinal as i64)),
            Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
            Cell::File(FileId(0)),
        ]) {
            column.0.push(to_bits(value).0);
        }
    }
    columns
}

#[test]
fn selected_join_keeps_duplicate_bag_order_and_decodes_only_projected_cells()
-> Result<(), OperatingError> {
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = facade::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![request()],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let right_origin = prepared.program().rules()[0].original_steps[1]
        .as_ref()
        .unwrap()
        .logical;
    let right_region = prepared.program().logical_regions()[right_origin.0];
    assert!(
        prepared.program().regions()[right_region.0]
            .selection
            .is_some(),
        "RHS must use the actual compiled stored selection"
    );
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("selected-join.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[true],
        services,
        b"",
        true,
        &mut symbols,
        None,
        None,
    )?;
    let left = edges(&[(99, 7)]);
    let right = edges(&[(3, 7), (1, 7), (3, 7)]);
    let read = |relation| match relation {
        Relation::SemanticChild => Ok(Rows::Stored(ColumnsView {
            kinds: kinds(relation),
            columns: &left,
        })),
        Relation::SyntaxChild => Ok(Rows::Stored(ColumnsView {
            kinds: kinds(relation),
            columns: &right,
        })),
        _ => Err(OperatingError::NotRead),
    };
    take_read_row_counts();
    let mut output = RefusingSink {
        reject: false,
        rows: vec![],
    };
    let outcome = forward(&mut file, &read, &mut symbols, &mut output)?;
    assert!(matches!(outcome, InvocationOutcome::Published(_)));
    assert_eq!(
        output
            .rows
            .iter()
            .flat_map(|row| nodes(row))
            .collect::<Vec<_>>(),
        [99, 3, 3, 99, 1, 1, 99, 3, 3]
    );
    // The streamed left provider reads its single canonical row. The three RHS
    // rows are decoded only by projected cell access, never as five-cell rows.
    assert_eq!(take_read_row_counts(), (1, 5));
    assert_eq!(
        file.route_observation(RuleIndex(0), StepId(1))?
            .registered_calls,
        0,
        "stored selection must be published instead of a materialized Select"
    );
    Ok(())
}
