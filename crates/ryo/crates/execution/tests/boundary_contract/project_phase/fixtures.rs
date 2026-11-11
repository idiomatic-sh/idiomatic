//! The support of the `project_phase` cases and of the groups nested in it.
use crate::boundary_fixture::fixtures::authored_request;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Relation},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
};
use ryo_values::domain::{
    plan_parts::types::{JoinColumn, Side},
    stored_facts::types::Bucket,
};

pub(crate) fn mixed_program(
    engine: &PrepareUseCaseImpl,
    relation: Relation,
) -> PreparedExecutionProgram {
    mixed_program_with(engine, relation, ObservationIdentity::Independent)
}

/// [`mixed_program`] under the observation identity `observations`.
pub(crate) fn mixed_program_with(
    engine: &PrepareUseCaseImpl,
    relation: Relation,
    observations: ObservationIdentity,
) -> PreparedExecutionProgram {
    let named = matches!(observations, ObservationIdentity::Named(_));
    let mut input = authored_request(if named { 2 } else { 1 });
    input.observations = observations;
    let file_column = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    crate::boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(u32::try_from(file_column).unwrap())],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::Relation(relation),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
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
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut input)[0].frontier =
        vec![Input::Step(StepId(0)), Input::Relation(relation)];
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .emit
        .step = StepId(2);
    if named {
        use ryo_values::domain::observations::types::{
            BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
            ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
        };
        use ryo_values::domain::plan_parts::types::ObservationBindingId;
        // A named program is a schedule: rule 0 publishes its answer as `files`, which rule 1 binds.
        let columns = vec![ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        }];
        crate::boundary_fixture::fixtures::plans(&mut input)[0]
            .observations
            .push(PlanObservation {
                name: ObservationName("files".into()),
                source: Input::Step(StepId(2)),
                columns: columns.clone(),
                scope: StepScope::CrossFile,
            });
        let consumer = &mut crate::boundary_fixture::fixtures::plans(&mut input)[1];
        consumer.steps = vec![PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::BoundObservation(ObservationBindingId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        }];
        consumer.frontier = vec![Input::BoundObservation(ObservationBindingId(0))];
        consumer.observation_inputs.push(BoundObservationInput {
            name: ObservationBindingName("files".into()),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        });
        let producer = input.rules[0].id.clone();
        input.rules[1].observations.push(ObservationBinding {
            name: ObservationBindingName("files".into()),
            columns,
            producers: vec![ObservationProducer::Output {
                rule: producer,
                output: ObservationName("files".into()),
            }],
        });
    }
    engine
        .prepare(input)
        .expect("compile actual mixed-phase request")
}

/// The kept frontier of rule 0 in file 0, captured as [`captured_in_file_0`] captures it, over `read`, keyed by its file, every
/// other rule undemanded.
pub(crate) fn captured_in_file_0_of<'facts>(
    request: &ryo_execution::inbound::use_cases::execution::interface::RequestExecution,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
    symbols: &mut Symbols,
) -> Vec<(
    FileId,
    ryo_execution_domain::domain::frontier_capture::types::CapturedFrontier,
)> {
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[
                    FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Forwarded,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    },
                    FileRuleIntent::Undemanded,
                ],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            symbols,
        )
        .expect("open file 0");
    let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
    let captures = file
        .capture_frontiers(
            RuleIndex(0),
            Bucket(0),
            symbols,
            CaptureContext {
                worker: WorkerNumber(0),
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words,
            },
        )
        .expect("capture the frontiers of rule 0");
    let _finished = file.finish();
    captures
        .into_iter()
        .filter_map(|capture| capture.outcome.ok().map(|kept| (FileId(0), kept)))
        .collect()
}

/// The kept frontier of each input of rule 0, captured in file 0 opened with its diagnostic collected and its frontiers captured,
/// whose `read` gives each relation its table in `rows`, with `publication` read in part.
pub(crate) fn captured_in_file_0(
    request: &ryo_execution::inbound::use_cases::execution::interface::RequestExecution,
    rows: &[(Relation, ryo_values::domain::rows::types::Table)],
    publication: &[ryo_execution::inbound::use_cases::execution::interface::Partial],
    symbols: &mut Symbols,
) -> Vec<ryo_execution_domain::domain::frontier_capture::types::CapturedFrontier> {
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |relation| {
        rows.iter()
            .find(|(each, _)| *each == relation)
            .map(|(_, table)| SourceRows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication,
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::DiagnosticAndObservations,
                }],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            symbols,
        )
        .expect("open file 0");
    let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
    let captures = file
        .capture_frontiers(
            RuleIndex(0),
            Bucket(0),
            symbols,
            CaptureContext {
                worker: WorkerNumber(0),
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words,
            },
        )
        .expect("capture the frontiers of rule 0");
    let _finished = file.finish();
    captures
        .into_iter()
        .map(|capture| capture.outcome.expect("a kept frontier"))
        .collect()
}
