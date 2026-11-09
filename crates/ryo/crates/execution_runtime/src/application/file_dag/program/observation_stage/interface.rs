//! What an observation stage offers a caller.
use crate::application::file_dag::frontiers::types::BoundInput;
use crate::application::file_dag::frontiers::types::InputReadiness;
use crate::application::file_dag::frontiers::types::PublicationSlot;
use crate::application::file_dag::frontiers::types::ReadinessProof;
use crate::application::file_dag::observations::types::RuleSelection;
use crate::application::file_dag::program::observation_stage::types::{
    AdmittedOutput, Install, ObservationBatches,
};
use crate::application::file_dag::program::project_phase::types::{
    OccurrenceNumber, PhasePartials, PhaseStop,
};
use crate::application::file_dag::program::rule_outcome::types::{
    ProjectOutputOutcome, RuleOutcome,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use crate::outbound::facts::interface::FactsFinish;
use ryo_execution_domain::domain::coverage::types::Covered;
use ryo_execution_domain::domain::coverage::types::FileObservationReadiness;
use ryo_execution_domain::domain::coverage::types::InapplicableFiles;
use ryo_execution_domain::domain::coverage::types::RefusedFiles;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::records::types::CrossPublicationTransfer;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::Bucket;
pub trait ObservationExecution {
    fn reserve(&mut self, rule: RuleIndex) -> Result<PublicationSlot, OperatingError>;
    fn bindings(&self, rule: RuleIndex) -> Result<Vec<BoundInput>, OperatingError>;
    fn binding_readiness(
        &self,
        rule: RuleIndex,
    ) -> Result<
        std::collections::BTreeMap<
            ryo_values::domain::plan_parts::types::ObservationBindingId,
            InputReadiness,
        >,
        OperatingError,
    >;
    /// The readiness of each retained frontier of `rule`, from the bindings each frontier depends on: a frontier is ready only when
    /// every one of them is published in this stage. The answer is what `FrontierDemand::Observations` takes.
    ///
    /// # Errors
    ///
    /// A rule the stage or the program did not declare.
    fn frontier_readiness(&self, rule: RuleIndex) -> Result<ReadinessProof, OperatingError>;
    fn arenas(&self) -> &[ryo_values::domain::stored_facts::types::ArenaBytes];
    fn finish(&mut self, rule: RuleIndex) -> Result<(), OperatingError>;
    fn finish_stage(self) -> Result<(), OperatingError>;
    /// Runs every declared occurrence of the stage's batches: the rules no occurrence names finished, the frontiers restored, then
    /// per batch each occurrence bound (its frontiers' readiness, its publication slot, its inputs) in declaration order, the bound
    /// ones evaluated with `before`, `partials` and `guard` and each outcome handed to `complete` with the batch's arenas and the
    /// worker its slot reserved, then each occurrence handed to `deliver` in declaration order, what it returns installed in its
    /// slot and its rule finished. Every callback is given the occurrence's place in `batches.occurrences`. The first error of a
    /// batch ends the stage.
    ///
    /// # Errors
    ///
    /// The first error of the stage: a rule it did not declare, a refused restoration, delivery, install or finish, or the guard's
    /// refusal.
    fn run_batches<H, T, E: From<OperatingError>>(
        self,
        batches: ObservationBatches<'_>,
        symbols: &mut Symbols,
        before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
        complete: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            &[ArenaBytes],
            Option<WorkerNumber>,
            &mut Symbols,
        ) -> Result<T, E>,
        deliver: impl FnMut(OccurrenceNumber, Result<T, PhaseStop<E>>) -> Result<Option<Install>, E>,
    ) -> Result<(), E>
    where
        Self: Sized;
}
pub trait ObservationPublication {
    fn install(
        &mut self,
        rule: RuleIndex,
        slot: PublicationSlot,
        transfer: CrossPublicationTransfer,
        publications: Vec<AdmittedOutput>,
    ) -> Result<(), OperatingError>;
    fn restore_frontiers(
        &mut self,
        facts: &dyn FactsFinish,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &[InapplicableFiles],
        refused: &[RefusedFiles],
        readiness: &[FileObservationReadiness],
        first_bucket: &[Bucket],
        selected: &[RuleSelection],
    ) -> Result<(), OperatingError>;
}
