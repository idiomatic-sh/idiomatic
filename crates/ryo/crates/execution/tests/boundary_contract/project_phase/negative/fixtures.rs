//! One project submission through the public phase, with the roots its guard was asked about.
use crate::boundary_fixture::fixtures::corpus;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ObservationStage;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProjectPhase;
use ryo_execution::inbound::use_cases::execution::interface::RequestExecution;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_relations::domain::relations::types::{CellKind, FileId};
use ryo_values::domain::{
    files::types::CorpusInput,
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName,
    },
    plan_parts::types::{Input, ObservationBindingId, StepScope},
    plans::types::{Operation, PlanStep},
    records::types::ObservationReadinessLayout,
    stored_facts::types::Digest,
};

/// The two public entries that accept a set of project roots.
#[derive(Clone, Copy)]
pub(crate) enum Entry {
    /// Ordinary roots through the request's project phase.
    Phase,
    /// Named roots through the observation stage that owns that phase.
    Stage,
}
pub(crate) enum Held<'a> {
    Phase(ProjectPhase<'a>),
    Stage(ObservationStage<'a>),
}

/// The prepared program of `rule_count` rules; the stage entry declares one closed named binding.
fn program(
    engine: &PrepareUseCaseImpl,
    entry: Entry,
    rule_count: usize,
) -> PreparedExecutionProgram {
    let mut authored = crate::boundary_fixture::fixtures::authored_request(rule_count);
    if matches!(entry, Entry::Stage) {
        // A named request needs a dependency schedule: rule 0 declares one closed empty binding.
        let columns = vec![ObservationColumn {
            name: ObservationColumnName("code".into()),
            kind: CellKind::Symbol,
        }];
        authored.rules[0].observations.push(ObservationBinding {
            name: ObservationBindingName("codes".into()),
            columns: columns.clone(),
            producers: vec![],
        });
        crate::boundary_fixture::fixtures::plans(&mut authored)[0]
            .observation_inputs
            .push(BoundObservationInput {
                name: ObservationBindingName("codes".into()),
                columns,
                scope: StepScope::CrossFile,
            });
        crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::BoundObservation(ObservationBindingId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        }];
        crate::boundary_fixture::fixtures::plans(&mut authored)[0].frontier =
            vec![Input::BoundObservation(ObservationBindingId(0))];
        authored.observations =
            ObservationIdentity::Named(ObservationReadinessLayout(Digest([7; 16])));
    }
    engine.prepare(authored).expect("valid public request")
}
/// Open the request and its phase (or observation stage) for `entry`, hand them to `check`, which releases what it was handed,
/// and release the request once `check` returns.
pub(crate) fn with_held<T>(
    engine: &PrepareUseCaseImpl,
    entry: Entry,
    rule_count: usize,
    check: impl FnOnce(&RequestExecution, Held<'_>, &[CorpusInput], &[Covered]) -> T,
) -> T {
    let prepared = program(engine, entry, rule_count);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let covered = [Covered {
        file: FileId(0),
        rules: (0..rule_count).map(RuleIndex).collect(),
        partial: vec![],
    }];
    let mut arenas =
        ryo_execution::inbound::use_cases::execution::interface::ObservationArenas::default();
    let held = match entry {
        Entry::Phase => Held::Phase(request.project_phase().unwrap()),
        Entry::Stage => Held::Stage(request.observations(&mut arenas).unwrap()),
    };
    let found = check(&request, held, &corpus, &covered);
    request.finish();
    found
}
