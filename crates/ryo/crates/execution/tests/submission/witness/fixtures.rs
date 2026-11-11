//! The support of the `witness` cases and of the groups nested in it.
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FactsWorker;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::Install;
use ryo_execution::inbound::use_cases::execution::interface::ObservationBatches;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
use ryo_execution::inbound::use_cases::execution::interface::ObserverPort;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_execution_domain::domain::frontier_capture::types::FrontierCapture;
use ryo_execution_domain::domain::row_emission::types::RowEmission;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileFinished;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileRules;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileRunInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
use ryo_execution_runtime::application::file_dag::program::observation_stage::types::ObservationStage;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::OccurrenceNumber;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhasePartials;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhaseStop;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectPhase;
use ryo_execution_runtime::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::{
    files::types::CorpusInput,
    plan_parts::types::InputNumber,
    stored_facts::types::{ArenaBytes, Bucket},
};

/// One variant per method of every entry trait, generated with its inventory so a variant cannot
/// be missing from `ALL`. `witness` below maps each trait method here, and the exhaustive `match`
/// in `controls::control` refuses to compile for a variant without a control.
macro_rules! methods {
    ($($method:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub(in super::super) enum Method { $($method),+ }
        pub(in super::super) const ALL: &[Method] = &[$(Method::$method),+];
    };
}
methods!(
    UseCasePrepare,
    UseCaseExecution,
    RequestObservations,
    RequestFinish,
    RequestOpenFile,
    RequestRunFile,
    RequestProjectPhase,
    RootsEvaluateRules,
    StageFinish,
    ObservedRunBatches,
    FileEmitRows,
    FileFinish,
    FileRule,
    FileCaptureFrontiers,
    FileCommitFrontier,
);
/// Its methods never run: implementing every method of each entry trait is what the compiler checks.
pub(in super::super) struct Witness;

pub(in super::super) fn implements_every_entry_trait<
    W: ExecutionUseCase
        + PrepareUseCase
        + ExecutionRequest
        + FileExecution
        + ProjectRootExecution
        + ProjectStage
        + ObservationExecution,
>(
    _: &W,
) {
}

pub(in super::super) fn witness(method: Method) -> ! {
    unreachable!("{method:?} is a compile-time witness only")
}

impl PrepareUseCase for Witness {
    fn prepare(&self, _: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError> {
        witness(Method::UseCasePrepare)
    }
}

impl ExecutionUseCase for Witness {
    fn execution(
        &self,
        _: &PreparedExecutionProgram,
        _: &[CorpusInput],
        _: Option<std::sync::Arc<dyn ObserverPort>>,
    ) -> RequestExecution {
        witness(Method::UseCaseExecution)
    }
}

impl ExecutionRequest for Witness {
    fn observations<'a>(
        &'a self,
        _: &'a mut Vec<ArenaBytes>,
    ) -> Result<ObservationStage<'a>, OperatingError> {
        witness(Method::RequestObservations)
    }
    fn finish(self) {
        witness(Method::RequestFinish)
    }
    fn open_file<'a, 'facts: 'a>(
        &'a self,
        _: FileInput<'a, 'facts>,
        _: &mut Symbols,
    ) -> Result<FileSession<'a, 'facts>, OperatingError> {
        witness(Method::RequestOpenFile)
    }
    fn run_file(
        &self,
        _: FileRunInput<'_>,
        _: &mut Symbols,
        _: CaptureContext<'_>,
        _: &dyn FactsWorker,
    ) -> FileRules {
        witness(Method::RequestRunFile)
    }
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError> {
        witness(Method::RequestProjectPhase)
    }
}

impl ProjectRootExecution for Witness {
    fn evaluate_rules<H, T, E>(
        &self,
        _: PhaseRules<'_>,
        _: &mut Symbols,
        _: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        _: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        _: impl FnMut(RuleIndex) -> Result<(), E>,
        _: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            &[ArenaBytes],
            Option<WorkerNumber>,
            &mut Symbols,
        ) -> Result<T, E>,
    ) -> ProjectBatch<Result<T, PhaseStop<E>>, E> {
        witness(Method::RootsEvaluateRules)
    }
}

impl ProjectStage for Witness {
    fn finish(self) -> Result<(), OperatingError> {
        witness(Method::StageFinish)
    }
}

impl ObservationExecution for Witness {
    fn run_batches<H, T, E: From<OperatingError>>(
        self,
        _: ObservationBatches<'_>,
        _: &mut Symbols,
        _: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        _: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        _: impl FnMut(RuleIndex) -> Result<(), E>,
        _: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            &[ArenaBytes],
            Option<WorkerNumber>,
            &mut Symbols,
        ) -> Result<T, E>,
        _: impl FnMut(OccurrenceNumber, Result<T, PhaseStop<E>>) -> Result<Option<Install>, E>,
    ) -> Result<(), E> {
        witness(Method::ObservedRunBatches)
    }
}

impl FileExecution for Witness {
    fn emit_rows(
        &mut self,
        _: RuleIndex,
        _: &mut Symbols,
        _: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<RowEmission, OperatingError> {
        witness(Method::FileEmitRows)
    }
    fn finish(self) -> Result<FileFinished, OperatingError> {
        witness(Method::FileFinish)
    }
    fn rule(
        &mut self,
        _: RuleIndex,
        _: RuleDemand,
        _: &mut Symbols,
    ) -> Result<RuleOutcome, OperatingError> {
        witness(Method::FileRule)
    }
    fn capture_frontiers(
        &mut self,
        _: RuleIndex,
        _: Bucket,
        _: &mut Symbols,
        _: CaptureContext<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError> {
        witness(Method::FileCaptureFrontiers)
    }
    fn commit_frontier(&mut self, _: RuleIndex, _: InputNumber) -> Result<(), OperatingError> {
        witness(Method::FileCommitFrontier)
    }
}
