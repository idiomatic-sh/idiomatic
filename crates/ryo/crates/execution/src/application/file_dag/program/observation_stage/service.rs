//! An observation stage: its reservations, installs and restored frontiers.
use crate::application::file_dag::frontiers::types::BoundInput;
use crate::application::file_dag::frontiers::types::InputReadiness;
use crate::application::file_dag::frontiers::types::PublicationSlot;
use crate::application::file_dag::observations::types::{AdmittedOutput, RuleSelection};
use crate::application::file_dag::program::observation_stage::interface::ObservationExecution;
use crate::application::file_dag::program::observation_stage::interface::ObservationPublication;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::project_phase::interface::ProjectRootExecution;
use crate::application::file_dag::program::project_phase::types::PhaseScope;
use crate::application::file_dag::program::project_phase::types::ProjectTask;
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use crate::domain::coverage::types::Covered;
use crate::domain::coverage::types::FileObservationReadiness;
use crate::domain::coverage::types::InapplicableFiles;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
use crate::domain::readiness::types::ReadinessCounts;
use crate::outbound::facts::interface::FactsFinish;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::records::types::CrossPublicationTransfer;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::Bucket;
impl ProjectRootExecution for ObservationStage<'_> {
    fn evaluate<E>(
        &self,
        tasks: Vec<ProjectTask<'_>>,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<RuleOutcome<ProjectOutputOutcome>, OperatingError>, E> {
        self.phase.evaluate(tasks, symbols, guard)
    }
    fn readiness_counts(&self) -> ReadinessCounts {
        self.phase.readiness_counts()
    }
}
impl ObservationExecution for ObservationStage<'_> {
    fn reserve(&mut self, rule: RuleIndex) -> Result<PublicationSlot, OperatingError> {
        self.publications.reserve(rule)
    }
    fn bindings(&self, rule: RuleIndex) -> Result<Vec<BoundInput>, OperatingError> {
        self.publications.bindings(rule)
    }
    fn binding_readiness(
        &self,
        rule: RuleIndex,
    ) -> Result<
        std::collections::BTreeMap<
            ryo_values::domain::plan_parts::types::ObservationBindingId,
            InputReadiness,
        >,
        OperatingError,
    > {
        self.publications.binding_readiness(rule)
    }
    fn arenas(&self) -> &[ryo_values::domain::stored_facts::types::ArenaBytes] {
        self.publications.arenas()
    }
    fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &InapplicableFiles,
    ) -> Result<PhaseScope, OperatingError> {
        self.phase
            .project_scope(rule, corpus, covered, skipped, inapplicable)
    }
    fn finish(&mut self, rule: RuleIndex) -> Result<(), OperatingError> {
        self.publications.finish(rule)
    }
    fn finish_stage(self) -> Result<(), OperatingError> {
        drop(self.publications);
        self.phase.finish()
    }
}
impl ObservationPublication for ObservationStage<'_> {
    fn install(
        &mut self,
        rule: RuleIndex,
        slot: PublicationSlot,
        transfer: CrossPublicationTransfer,
        publications: Vec<AdmittedOutput>,
    ) -> Result<(), OperatingError> {
        self.publications
            .install(rule, slot, transfer, publications)
    }
    fn restore_frontiers(
        &mut self,
        facts: &dyn FactsFinish,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &[InapplicableFiles],
        frames: &[FileObservationReadiness],
        first_bucket: &[Bucket],
        selected: &[RuleSelection],
    ) -> Result<(), OperatingError> {
        crate::application::file_dag::observations::restore::service::restore_frontiers(
            &mut self.publications,
            &self.phase.request.program,
            &self.phase.request.services,
            &self.phase.publication,
            facts,
            corpus,
            covered,
            skipped,
            inapplicable,
            frames,
            first_bucket,
            selected,
        )
    }
}
