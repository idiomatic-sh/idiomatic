//! One project publication generation shared by every ready batch, and what a caller hands it and is handed.
use crate::application::file_dag::program::project_phase::service::ProjectPhase;
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
/// One output of a rule with the admitted publication it delivers.
///
/// # Examples
///
/// ```
/// // Its publication's fields are crate-private, built by admission: the field's type, checked.
/// use ryo_execution_runtime::application::file_dag::program::observation_stage::types::AdmittedOutput;
/// use ryo_values::domain::records::types::ObservationIndex;
/// fn output_of(admitted: &AdmittedOutput) -> ObservationIndex {
///     admitted.output
/// }
/// ```
pub struct AdmittedOutput {
    pub output: ObservationIndex,
    pub publication:
        crate::application::file_dag::program::observed_publication::service::ObservedPublication,
}
/// One ready batch of the program, by its place among the batches.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::observation_stage::types::BatchNumber;
/// assert_ne!(BatchNumber(1), BatchNumber(0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatchNumber(pub usize);
/// One declared occurrence of an observation stage: the batch it runs in and its rule.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_runtime::application::file_dag::program::observation_stage::types::{BatchNumber, BatchOccurrence};
/// let occurrence = BatchOccurrence { batch: BatchNumber(0), rule: RuleIndex(2) };
/// assert_eq!(occurrence.rule, RuleIndex(2));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatchOccurrence {
    /// The batch the occurrence runs in.
    pub batch: BatchNumber,
    /// The occurrence's rule.
    pub rule: RuleIndex,
}
/// What the batches of an observation stage read: every declared occurrence with its batch, and the corpus and facts their
/// frontiers are read from. The arenas are the stage's own, read once per batch.
///
/// # Examples
///
/// ```
/// // It borrows a run's facts, readiness and corpus, which no public constructor builds: the field's type, checked.
/// use ryo_execution_runtime::application::file_dag::program::observation_stage::types::{BatchOccurrence, ObservationBatches};
/// fn occurrences<'a>(batches: &ObservationBatches<'a>) -> &'a [BatchOccurrence] {
///     batches.occurrences
/// }
/// ```
pub struct ObservationBatches<'a> {
    /// Every declared occurrence, in the order the caller numbers them: per batch, per rule of the batch, per declared occurrence
    /// of that rule.
    pub occurrences: &'a [BatchOccurrence],
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
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::observation_stage::types::Install;
/// use ryo_relations::domain::relations::types::WorkerNumber;
/// use ryo_values::domain::records::types::CrossPublicationTransfer;
/// use ryo_values::domain::stored_facts::types::ArenaBytes;
/// let install = Install {
///     transfer: CrossPublicationTransfer { worker: WorkerNumber(0), arena: ArenaBytes(Vec::new()), roots: Vec::new() },
///     publications: Vec::new(),
/// };
/// assert!(install.publications.is_empty());
/// ```
pub struct Install {
    /// What the occurrence transfers to the stage's arenas.
    pub transfer: CrossPublicationTransfer,
    /// The admitted publication of each named output it delivers.
    pub publications: Vec<AdmittedOutput>,
}
