//! The witness that implements every method of each entry trait.
use ryo_execution::inbound::use_cases::execution::{
    interface::{
        ExecutionRequest, InvocationStateCounts, ObservationExecution, ProjectRootExecution,
        ProjectStage, ReadinessCounts,
    },
    types::*,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::{
    files::types::CorpusInput, observations::types::ObservationBindingId, runs::types::Skipped,
    stored_facts::types::ArenaBytes,
};

/// One variant per method of every entry trait. `witness` below maps each trait method here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Method {
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
}
pub(super) const ALL: [Method; 17] = [
    Method::RequestObservations,
    Method::RequestPrepareFrontiers,
    Method::RequestFinish,
    Method::RequestOpenFile,
    Method::RequestProjectPhase,
    Method::RootsEvaluate,
    Method::RootsReadinessCounts,
    Method::StageStateCounts,
    Method::StageProjectScope,
    Method::StageFinish,
    Method::ObservedReserve,
    Method::ObservedBindings,
    Method::ObservedBindingReadiness,
    Method::ObservedArenas,
    Method::ObservedProjectScope,
    Method::ObservedFinish,
    Method::ObservedFinishStage,
];
/// Its methods never run: implementing every method of each entry trait is what the compiler checks.
pub(super) struct Witness;
pub(super) fn implements_every_entry_trait<
    W: ExecutionRequest + ProjectRootExecution + ProjectStage + ObservationExecution,
>(
    _: &W,
) {
}
pub(super) fn witness(method: Method) -> ! {
    unreachable!("{method:?} is a compile-time witness only")
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
