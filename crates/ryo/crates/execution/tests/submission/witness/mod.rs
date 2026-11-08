//! The witness that implements every method of each entry trait.
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::InvocationStateCounts;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
use ryo_execution::inbound::use_cases::execution::interface::ObservationPublication;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::ReadinessCounts;
use ryo_execution::inbound::use_cases::execution::interface::WalkCounts;
use ryo_execution_runtime::application::file_dag::frontiers::errors::PreparationError;
use ryo_execution_runtime::application::file_dag::frontiers::types::BoundInput;
use ryo_execution_runtime::application::file_dag::frontiers::types::ReadinessProof;
use ryo_execution_runtime::application::file_dag::frontiers::types::PreparedFrontiers;
use ryo_execution_runtime::application::file_dag::frontiers::types::PublicationSlot;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileFinished;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileRules;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileRunInput;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
use ryo_execution_runtime::application::file_dag::program::observation_stage::types::ObservationStage;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectPhase;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhaseScope;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::FrontierRequest;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::OccurrenceNumber;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhaseOccurrences;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhaseStop;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhasePartials;
use ryo_execution_runtime::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::FileObservationReadiness;
use ryo_execution::inbound::use_cases::execution::interface::FrontierCapture;
use ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles;
use ryo_execution::inbound::use_cases::execution::interface::LogicalCounts;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RowEmission;
use ryo_execution::inbound::use_cases::execution::interface::RuleDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::{
    files::types::CorpusInput,
    plan_parts::types::{InputNumber, ObservationBindingId},
    runs::types::Skipped,
    stored_facts::types::{ArenaBytes, Bucket},
};

/// One variant per method of every entry trait, generated with its inventory so a variant cannot
/// be missing from `ALL`; `TRAIT_METHODS` counts the methods of each trait in the same list, and
/// `controls::every_trait_method_has_its_own_variant` compares them to the trait sources. `witness` below maps each trait method here, and the exhaustive `match`
/// in `controls::control` refuses to compile for a variant without a control.
macro_rules! methods {
    ($($method:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub(super) enum Method { $($method),+ }
        pub(super) const ALL: &[Method] = &[$(Method::$method),+];
    };
}
methods!(
    UseCasePrepare,
    UseCaseExecution,
    RequestObservations,
    RequestPrepareFrontiers,
    RequestFinish,
    RequestOpenFile,
    RequestRunFile,
    RequestProjectPhase,
    RootsEvaluateOccurrences,
    RootsReadinessCounts,
    StageStateCounts,
    StageProjectScope,
    StageFinish,
    ObservedReserve,
    ObservedBindings,
    ObservedBindingReadiness,
    ObservedFrontierReadiness,
    ObservedArenas,
    ObservedProjectScope,
    ObservedFinish,
    ObservedFinishStage,
    PublicationInstall,
    PublicationRestoreFrontiers,
    FileReadinessCounts,
    FileWalkCounts,
    FileEmitRows,
    FileFinish,
    FileRule,
    FileCaptureFrontiers,
    FileCommitFrontier,
    FileCounts,
);
/// Its methods never run: implementing every method of each entry trait is what the compiler checks.
pub(super) struct Witness;
pub(super) fn implements_every_entry_trait<
    W: ExecutionUseCase
        + PrepareUseCase
        + ExecutionRequest
        + FileExecution
        + ProjectRootExecution
        + ProjectStage
        + ObservationExecution
        + ObservationPublication,
>(
    _: &W,
) {
}
pub(super) fn witness(method: Method) -> ! {
    unreachable!("{method:?} is a compile-time witness only")
}
impl PrepareUseCase for Witness {
    fn prepare(&self, _: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError> {
        witness(Method::UseCasePrepare)
    }
}
impl ExecutionUseCase for Witness {
    fn execution(&self, _: &PreparedExecutionProgram, _: &[CorpusInput]) -> RequestExecution {
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
    fn prepare_frontiers(
        &self,
        _: FrontierRequest<'_>,
        _: &mut Symbols,
    ) -> Result<PreparedFrontiers, PreparationError> {
        witness(Method::RequestPrepareFrontiers)
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
    fn evaluate_occurrences<H, T, E>(
        &self,
        _: PhaseOccurrences<'_>,
        _: &mut Symbols,
        _: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        _: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        _: impl FnMut(RuleIndex) -> Result<(), E>,
        _: impl FnMut(OccurrenceNumber, RuleOutcome<ProjectOutputOutcome>, PublicationSlot, &mut Symbols) -> Result<T, E>,
    ) -> ProjectBatch<Result<T, PhaseStop<E>>, E> {
        witness(Method::RootsEvaluateOccurrences)
    }
    fn readiness_counts(&self) -> ReadinessCounts {
        witness(Method::RootsReadinessCounts)
    }
}
impl ProjectStage for Witness {
    fn state_counts(&self) -> InvocationStateCounts {
        witness(Method::StageStateCounts)
    }
    fn project_scope(
        &self,
        _: RuleIndex,
        _: &[CorpusInput],
        _: &[Covered],
        _: &[Skipped],
        _: &ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles,
    ) -> Result<PhaseScope, OperatingError> {
        witness(Method::StageProjectScope)
    }
    fn finish(self) -> Result<(), OperatingError> {
        witness(Method::StageFinish)
    }
}
impl ObservationExecution for Witness {
    fn reserve(&mut self, _: RuleIndex) -> Result<PublicationSlot, OperatingError> {
        witness(Method::ObservedReserve)
    }
    fn bindings(&self, _: RuleIndex) -> Result<Vec<BoundInput>, OperatingError> {
        witness(Method::ObservedBindings)
    }
    fn binding_readiness(
        &self,
        _: RuleIndex,
    ) -> Result<std::collections::BTreeMap<ObservationBindingId, ryo_execution_runtime::application::file_dag::frontiers::types::InputReadiness>, OperatingError> {
        witness(Method::ObservedBindingReadiness)
    }
    fn frontier_readiness(&self, _: RuleIndex) -> Result<ReadinessProof, OperatingError> {
        witness(Method::ObservedFrontierReadiness)
    }
    fn arenas(&self) -> &[ArenaBytes] {
        witness(Method::ObservedArenas)
    }
    fn project_scope(
        &self,
        _: RuleIndex,
        _: &[CorpusInput],
        _: &[Covered],
        _: &[Skipped],
        _: &ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles,
    ) -> Result<PhaseScope, OperatingError> {
        witness(Method::ObservedProjectScope)
    }
    fn finish(&mut self, _: RuleIndex) -> Result<(), OperatingError> {
        witness(Method::ObservedFinish)
    }
    fn finish_stage(self) -> Result<(), OperatingError> {
        witness(Method::ObservedFinishStage)
    }
}
impl FileExecution for Witness {
    fn readiness_counts(&self) -> ReadinessCounts {
        witness(Method::FileReadinessCounts)
    }
    fn walk_counts(&self) -> WalkCounts {
        witness(Method::FileWalkCounts)
    }
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
    fn counts(&self, _: RuleIndex) -> Result<LogicalCounts, OperatingError> {
        witness(Method::FileCounts)
    }
}
impl ObservationPublication for Witness {
    fn install(
        &mut self,
        _: RuleIndex,
        _: PublicationSlot,
        _: ryo_values::domain::records::types::CrossPublicationTransfer,
        _: Vec<ryo_execution_runtime::application::file_dag::observations::types::AdmittedOutput>,
    ) -> Result<(), OperatingError> {
        witness(Method::PublicationInstall)
    }
    fn restore_frontiers(
        &mut self,
        _: &dyn FactsFinish,
        _: &[CorpusInput],
        _: &[Covered],
        _: &[Skipped],
        _: &[InapplicableFiles],
        _: &[FileObservationReadiness],
        _: &[Bucket],
        _: &[ryo_execution_runtime::application::file_dag::observations::types::RuleSelection],
    ) -> Result<(), OperatingError> {
        witness(Method::PublicationRestoreFrontiers)
    }
}
