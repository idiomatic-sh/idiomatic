//! What an observation stage offers a caller.
use crate::application::file_dag::frontiers::types::BoundInput;
use crate::application::file_dag::frontiers::types::PublicationSlot;
use crate::application::file_dag::program::project_phase::types::PhaseScope;
use crate::application::file_dag::program::observed_publication::types::ObservedPublication;
use crate::domain::coverage::types::Covered;
use crate::domain::coverage::types::FileObservationReadiness;
use crate::domain::coverage::types::InapplicableFiles;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
use crate::outbound::facts::interface::FactsFinish;
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
            bool,
        >,
        OperatingError,
    >;
    fn arenas(&self) -> &[ryo_values::domain::stored_facts::types::ArenaBytes];
    fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &InapplicableFiles,
    ) -> Result<PhaseScope, OperatingError>;
    fn finish(&mut self, rule: RuleIndex) -> Result<(), OperatingError>;
    fn finish_stage(self) -> Result<(), OperatingError>;
}
pub trait ObservationPublication {
    fn install(
        &mut self,
        rule: RuleIndex,
        slot: PublicationSlot,
        transfer: CrossPublicationTransfer,
        publications: Vec<(
            ryo_values::domain::records::types::ObservationIndex,
            ObservedPublication,
        )>,
    ) -> Result<(), OperatingError>;
    fn restore_frontiers(
        &mut self,
        facts: &dyn FactsFinish,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &[InapplicableFiles],
        readiness: &[FileObservationReadiness],
        first_bucket: &[Bucket],
        selected: &[bool],
    ) -> Result<(), OperatingError>;
}
