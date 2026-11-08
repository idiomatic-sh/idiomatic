//! One project phase: its scope, its inputs and its tasks.
use crate::application::file_dag::frontiers::types::BoundInput;
use crate::application::file_dag::frontiers::errors::PreparationError;
use crate::application::file_dag::frontiers::types::ReadinessProof;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_execution_domain::domain::coverage::types::Covered;
use ryo_execution_domain::domain::coverage::types::InapplicableFiles;
use ryo_execution_domain::domain::coverage::types::Partials;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::interface::FactsWorker;
/// One project-phase publication/interpreter and symbol generation, shared across rules.
pub struct ProjectPhase<'a> {
    pub(crate) request: &'a RequestExecution,
    pub(crate) publication:
        std::rc::Rc<crate::application::file_dag::program::facade_runtime::types::PublicationScope>,
}
pub struct PhaseScope {
    pub(crate) required: crate::application::file_dag::program::admission::RequiredPartitions,
    pub(crate) publication:
        std::rc::Rc<crate::application::file_dag::program::facade_runtime::types::PublicationScope>,
}
pub struct ProjectInput<'a> {
    pub scope: &'a PhaseScope,
    pub evaluation: &'a Partials,
    pub publication: &'a Partials,
    pub ordinary: PreparedFrontiers,
    pub bound: &'a [BoundInput],
    pub arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
}
/// Actual, already prepared inputs and declared roots for one project task.
pub struct ProjectTask<'a> {
    pub rule: RuleIndex,
    pub demand: RuleDemand,
    pub input: ProjectInput<'a>,
}
pub enum FrontierDemand<'a> {
    Diagnostic,
    Observations { ready: &'a ReadinessProof },
}
pub struct FrontierRequest<'a> {
    pub demand: FrontierDemand<'a>,
    pub scope: &'a PhaseScope,
    pub rule: RuleIndex,
    pub first: ryo_values::domain::stored_facts::types::Bucket,
    pub facts: &'a dyn FactsFinish,
    pub project: &'a dyn FactsWorker,
    pub project_symbols: &'a ryo_language::domain::symbols::types::Symbols,
}
/// The place of one declared occurrence among the occurrences of its phase, counted from zero in declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OccurrenceNumber(pub usize);
/// What one ordinary project phase reads: its declared occurrences, the corpus they are scoped over and the facts their frontiers
/// are read from.
pub struct OrdinaryPhase<'a> {
    /// The rule of each declared occurrence, in declaration order; a rule may occur more than once.
    pub occurrences: &'a [RuleIndex],
    /// The first bucket of each rule's frontier, by rule.
    pub first: &'a [ryo_values::domain::stored_facts::types::Bucket],
    /// The files of the request.
    pub corpus: &'a [CorpusInput],
    /// The files whose relations the rules read in part.
    pub covered: &'a [Covered],
    /// The files each rule skipped.
    pub skipped: &'a [Skipped],
    /// The files each rule does not apply to, by rule.
    pub inapplicable: &'a [InapplicableFiles],
    /// Finishes the facts the frontiers are read from.
    pub facts: &'a dyn FactsFinish,
    /// The project's worker over the facts.
    pub project: &'a dyn FactsWorker,
    /// The symbols the project facts name.
    pub project_symbols: &'a Symbols,
    /// The arenas of the invocation.
    pub arenas: &'a [ArenaBytes],
}
impl std::fmt::Debug for OrdinaryPhase<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
/// The partials one occurrence is evaluated with: those its diagnostic sees and those its publication sees.
#[derive(Debug)]
pub struct PhasePartials {
    /// The partials the diagnostic evaluation sees.
    pub evaluation: Partials,
    /// The partials the publication sees.
    pub publication: Partials,
}
/// Why one occurrence of an ordinary project phase has no completed outcome.
#[derive(Debug)]
pub enum PhaseStop<E> {
    /// The caller's own refusal: its budget, its partials or its completion.
    Caller(E),
    /// The occurrence's rule has no first bucket for its frontier.
    MissingFirstBucket,
    /// The engine refused the occurrence's scope or its evaluation.
    Operating(OperatingError),
    /// The occurrence's frontiers could not be prepared.
    Frontiers(PreparationError),
    /// The engine answered fewer outcomes than prepared occurrences, with no refusal to explain it.
    Unevaluated,
}
/// What one ordinary project phase came to: each occurrence's completion in declaration order up to the guard's refusal, that
/// refusal, and the release of the phase.
#[derive(Debug)]
pub struct OrdinaryRules<T, E> {
    /// Each occurrence's completion, or why it has none; occurrences after the guard's refusal are absent.
    pub occurrences: Vec<Result<T, PhaseStop<E>>>,
    /// The guard's refusal, when it stopped the batch.
    pub refused: Option<E>,
    /// The release of the phase, made once every occurrence was completed.
    pub finished: Result<(), OperatingError>,
}
/// One occurrence of an ordinary phase once prepared: its scope and its frontiers.
pub(crate) struct PreparedOccurrence {
    pub(crate) scope: PhaseScope,
    pub(crate) frontier: PreparedFrontiers,
}
/// One prepared occurrence once the caller gave its partials, its frontiers handed on to its task.
pub(crate) struct ReadyOccurrence {
    pub(crate) scope: PhaseScope,
    pub(crate) partials: PhasePartials,
}
