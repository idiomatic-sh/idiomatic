//! The witness that implements every method of each entry trait.
use ryo_execution::application::file_dag::program::request_execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::InvocationStateCounts;
use ryo_execution::application::file_dag::program::observation_stage::interface::ObservationExecution;
use ryo_execution::application::file_dag::program::observation_stage::interface::ObservationPublication;
use ryo_execution::application::file_dag::program::project_phase::interface::ProjectRootExecution;
use ryo_execution::application::file_dag::program::project_phase::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::ReadinessCounts;
use ryo_execution::inbound::use_cases::execution::interface::WalkCounts;
use ryo_execution::application::file_dag::frontiers::errors::PreparationError;
use ryo_execution::application::file_dag::frontiers::types::BoundInput;
use ryo_execution::application::file_dag::frontiers::types::PreparedFrontiers;
use ryo_execution::application::file_dag::frontiers::types::PublicationSlot;
use ryo_execution::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution::application::file_dag::program::file_session::types::FileFinished;
use ryo_execution::application::file_dag::program::file_session::types::FileInput;
use ryo_execution::application::file_dag::program::file_session::types::FileSession;
use ryo_execution::application::file_dag::program::observation_stage::types::ObservationStage;
use ryo_execution::application::file_dag::program::observed_publication::types::ObservedPublication;
use ryo_execution::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution::application::file_dag::program::project_phase::types::ProjectPhase;
use ryo_execution::application::file_dag::program::project_phase::types::ProjectScope;
use ryo_execution::application::file_dag::program::project_phase::types::ProjectTask;
use ryo_execution::application::file_dag::program::request_execution::types::FrontierRequest;
use ryo_execution::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_execution::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution::application::file_dag::program::rule_outcome::types::ProjectRuleOutcome;
use ryo_execution::application::file_dag::program::rule_outcome::types::RuleOutcome;
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
    RequestProjectPhase,
    RootsEvaluate,
    RootsReadinessCounts,
    StageStateCounts,
    StageProjectScope,
    StageFinish,
    ObservedReserve,
    ObservedBindings,
    ObservedBindingReadiness,
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
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError> {
        witness(Method::RequestProjectPhase)
    }
}
impl ProjectRootExecution for Witness {
    fn evaluate<E>(
        &self,
        _: Vec<ProjectTask<'_>>,
        _: &mut Symbols,
        _: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<ProjectRuleOutcome, OperatingError>, E> {
        witness(Method::RootsEvaluate)
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
    ) -> Result<ProjectScope, OperatingError> {
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
    ) -> Result<std::collections::BTreeMap<ObservationBindingId, bool>, OperatingError> {
        witness(Method::ObservedBindingReadiness)
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
    ) -> Result<ProjectScope, OperatingError> {
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
        _: Vec<(
            ryo_values::domain::records::types::ObservationIndex,
            ObservedPublication,
        )>,
    ) -> Result<(), OperatingError> {
        witness(Method::PublicationInstall)
    }
    fn restore_frontiers(
        &mut self,
        _: &dyn facts::FactsFinish,
        _: &[CorpusInput],
        _: &[Covered],
        _: &[Skipped],
        _: &[InapplicableFiles],
        _: &[FileObservationReadiness],
        _: &[Bucket],
        _: &[bool],
    ) -> Result<(), OperatingError> {
        witness(Method::PublicationRestoreFrontiers)
    }
}
