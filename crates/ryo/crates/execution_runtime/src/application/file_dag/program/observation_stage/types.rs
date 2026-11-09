//! One project publication generation shared by every ready batch, and what a caller hands it and is handed.
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::interface::FactsWorker;
use ryo_execution_domain::domain::coverage::types::Covered;
use ryo_execution_domain::domain::coverage::types::FileObservationReadiness;
use ryo_execution_domain::domain::coverage::types::InapplicableFiles;
use ryo_execution_domain::domain::coverage::types::RefusedFiles;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::records::types::CrossPublicationTransfer;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::Bucket;
/// One project publication generation shared by every ready batch.
pub struct ObservationStage<'a> {
    pub(crate) phase: ProjectPhase<'a>,
    pub(crate) publications:
        crate::application::file_dag::observations::types::CrossPublications<'a>,
}
/// One output of a rule with the admitted publication it delivers.
pub struct AdmittedOutput {
    pub output: ObservationIndex,
    pub publication:
        crate::application::file_dag::program::observed_publication::types::ObservedPublication,
}
/// One ready batch of the program, by its place among the batches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatchNumber(pub usize);
/// What the batches of an observation stage read: every declared occurrence with its batch, and the corpus and facts their
/// frontiers are read from. The arenas are the stage's own, read once per batch.
pub struct ObservationBatches<'a> {
    /// Every declared occurrence, in the order the caller numbers them: per batch, per rule of the batch, per declared occurrence
    /// of that rule.
    pub occurrences: &'a [(BatchNumber, RuleIndex)],
    /// What each file's run observed of the producers that feed named observations.
    pub readiness: &'a [FileObservationReadiness],
    /// Every rule of the program; those named in `occurrences` are selected, the others finished first.
    pub rules: &'a [RuleIndex],
    /// The first bucket of each rule's frontier, by rule.
    pub first: &'a [Bucket],
    /// The files of the request.
    pub corpus: &'a [CorpusInput],
    /// The files whose relations the rules read in part.
    pub covered: &'a [Covered],
    /// The files each rule skipped.
    pub skipped: &'a [Skipped],
    /// The files each rule does not apply to, by rule.
    pub inapplicable: &'a [InapplicableFiles],
    /// The files each rule was refused on, by rule.
    pub refused: &'a [RefusedFiles],
    /// Finishes the facts the frontiers are read from.
    pub facts: &'a dyn FactsFinish,
    /// The project's worker over the facts.
    pub project: &'a dyn FactsWorker,
    /// The symbols the project facts name.
    pub project_symbols: &'a Symbols,
}
impl std::fmt::Debug for ObservationBatches<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
/// What one delivered occurrence hands the stage to install in the slot it reserved: its transfer and its admitted outputs.
pub struct Install {
    pub transfer: CrossPublicationTransfer,
    pub publications: Vec<AdmittedOutput>,
}
