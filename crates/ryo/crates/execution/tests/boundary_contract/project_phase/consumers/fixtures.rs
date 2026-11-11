//! A project producer of one named output and two consumers that bind it.
use crate::boundary_fixture::fixtures::authored_request;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, Relation},
};
use ryo_values::domain::{
    observations::types::*, plan_parts::types::ObservationBindingId,
    records::types::ObservationReadinessLayout, stored_facts::types::Digest,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
};

/// Rule 0 publishes the files of `ModuleIdentity` as its named output `files`; rules 1 and 2 each select that output through a
/// binding of their own.
pub(crate) fn producer_and_consumers(engine: &PrepareUseCaseImpl) -> PreparedExecutionProgram {
    let mut authored = authored_request(3);
    let relation = Relation::ModuleIdentity;
    let file_column = kinds(relation)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("file".into()),
        kind: CellKind::File,
    }];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::Relation(relation),
            comparisons: vec![],
            kept: vec![ColumnIndex(u32::try_from(file_column).unwrap())],
        },
    }];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].frontier =
        vec![Input::Relation(relation)];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].observations =
        vec![PlanObservation {
            name: ObservationName("files".into()),
            source: Input::Step(StepId(0)),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        }];
    let producer = authored.rules[0].id.clone();
    for consumer in [1, 2] {
        crate::boundary_fixture::fixtures::plans(&mut authored)[consumer].steps = vec![PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::BoundObservation(ObservationBindingId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        }];
        crate::boundary_fixture::fixtures::plans(&mut authored)[consumer].frontier =
            vec![Input::BoundObservation(ObservationBindingId(0))];
        crate::boundary_fixture::fixtures::plans(&mut authored)[consumer].observation_inputs =
            vec![BoundObservationInput {
                name: ObservationBindingName("files".into()),
                columns: columns.clone(),
                scope: StepScope::CrossFile,
            }];
        authored.rules[consumer].observations = vec![ObservationBinding {
            name: ObservationBindingName("files".into()),
            columns: columns.clone(),
            producers: vec![ObservationProducer::Output {
                rule: producer.clone(),
                output: ObservationName("files".into()),
            }],
        }];
    }
    authored.observations =
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([71; 16])));
    engine.prepare(authored).unwrap()
}
