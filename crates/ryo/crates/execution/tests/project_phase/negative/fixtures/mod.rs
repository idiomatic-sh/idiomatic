//! One project submission through the public phase, with the roots its guard was asked about.
use ryo_execution_runtime::outbound::facts::types::ColumnsMut;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_execution_runtime::outbound::facts::types::KeptBucket;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::outbound::facts::types::WordsToKeep;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use crate::boundary_fixture::corpus;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution_runtime::application::file_dag::program::observation_stage::types::ObservationStage;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectPhase;
use ryo_execution_runtime::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
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
    stored_facts::types::{Bucket, Digest},
};

/// No compiled rule in these programs has a file frontier or a project relation.
pub(crate) struct NoFacts;
impl FactsFinish for NoFacts {
    fn kept(&self, _: Bucket) -> Result<KeptBucket, FactsError> {
        panic!("a submission control has no file frontier to fetch")
    }
}
impl FactsWorker for NoFacts {
    fn resident_stability(&self) -> ResidentStability {
        ResidentStability::Stable
    }
    fn rows(
        &self,
        _: ryo_relations::domain::relations::types::Relation,
    ) -> Result<ryo_relations::domain::relations::types::ColumnsView<'_>, FactsError> {
        panic!("a submission control has no project relation to read")
    }
    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        panic!("a submission control writes no facts")
    }
    fn keep(&self, _: Bucket, _: WordsToKeep<'_>) -> Result<(), FactsError> {
        panic!("a submission control writes no facts")
    }
    fn release(&mut self) -> Result<(), FactsError> {
        panic!("a submission control writes no facts")
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        panic!("a submission control writes no facts")
    }
}

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
impl Held<'_> {
    fn finish(self) {
        match self {
            Held::Phase(phase) => phase.finish().unwrap(),
            Held::Stage(stage) => stage.finish_stage().unwrap(),
        }
    }
}

/// The prepared program of `rule_count` rules; the stage entry declares one closed named binding.
fn program(
    engine: &PrepareUseCaseImpl,
    entry: Entry,
    rule_count: usize,
) -> PreparedExecutionProgram {
    let mut authored = crate::boundary_fixture::authored_request(rule_count);
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
        crate::boundary_fixture::plans(&mut authored)[0]
            .observation_inputs
            .push(BoundObservationInput {
                name: ObservationBindingName("codes".into()),
                columns,
                scope: StepScope::CrossFile,
            });
        crate::boundary_fixture::plans(&mut authored)[0].steps = vec![PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::BoundObservation(ObservationBindingId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        }];
        crate::boundary_fixture::plans(&mut authored)[0].frontier = vec![Input::BoundObservation(ObservationBindingId(0))];
        authored.observations =
            ObservationIdentity::Named(ObservationReadinessLayout(Digest([7; 16])));
    }
    engine.prepare(authored).expect("valid public request")
}
/// Open the request and its phase (or observation stage) for `entry`, hand them to `check`,
/// and release everything once `check` has dropped every scope it made.
pub(crate) fn with_held<T>(
    engine: &PrepareUseCaseImpl,
    entry: Entry,
    rule_count: usize,
    check: impl FnOnce(&RequestExecution, &mut Held<'_>, &[CorpusInput], &[Covered]) -> T,
) -> T {
    let prepared = program(engine, entry, rule_count);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
    let covered = [Covered {
        file: FileId(0),
        rules: (0..rule_count).map(RuleIndex).collect(),
        partial: vec![],
    }];
    let mut arenas = Vec::new();
    let mut held = match entry {
        Entry::Phase => Held::Phase(request.project_phase().unwrap()),
        Entry::Stage => Held::Stage(request.observations(&mut arenas).unwrap()),
    };
    let found = check(&request, &mut held, &corpus, &covered);
    held.finish();
    request.finish();
    found
}
/// A compiled rule with no file frontier to fetch.
pub(crate) struct NoFileFrontiers;
impl FactsFinish for NoFileFrontiers {
    fn kept(&self, _: Bucket) -> Result<KeptBucket, FactsError> {
        panic!("this compiled rule has no file frontier to fetch")
    }
}
