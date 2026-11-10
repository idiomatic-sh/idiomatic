//! One project phase: its scope, its inputs and its tasks.
use crate::application::file_dag::frontiers::errors::PreparationError;
use crate::application::file_dag::frontiers::types::BoundInput;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::frontiers::types::PublicationSlot;
use crate::application::file_dag::frontiers::types::ReadinessProof;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::interface::FactsWorker;
use ryo_execution_domain::domain::coverage::types::Covered;
use ryo_execution_domain::domain::coverage::types::InapplicableFiles;
use ryo_execution_domain::domain::coverage::types::Partials;
use ryo_execution_domain::domain::coverage::types::RefusedFiles;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::ArenaBytes;
/// One project-phase publication/interpreter and symbol generation, shared across rules.
pub struct ProjectPhase<'a> {
    pub(crate) request: &'a RequestExecution,
    pub(crate) publication:
        std::rc::Rc<crate::application::file_dag::program::facade_runtime::types::PublicationScope>,
}
/// The files one occurrence's rule reads in its phase, sharing the phase's publication: crate-private, as the scoping is.
pub(crate) struct PhaseScope {
    pub(crate) required: crate::application::file_dag::program::admission::RequiredPartitions,
    pub(crate) publication:
        std::rc::Rc<crate::application::file_dag::program::facade_runtime::types::PublicationScope>,
}
/// The prepared inputs of one project task: crate-private, as the root evaluation that takes them is.
pub(crate) struct ProjectInput<'a> {
    pub(crate) scope: &'a PhaseScope,
    pub(crate) evaluation: &'a Partials,
    pub(crate) publication: &'a Partials,
    pub(crate) ordinary: PreparedFrontiers,
    pub(crate) bound: &'a [BoundInput],
    pub(crate) arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
}
/// Actual, already prepared inputs and declared roots for one project task: crate-private, so no caller outside the runtime
/// assembles a root of its own.
pub(crate) struct ProjectTask<'a> {
    pub(crate) rule: RuleIndex,
    pub(crate) demand: RuleDemand,
    pub(crate) input: ProjectInput<'a>,
}
#[derive(Clone, Copy)]
pub(crate) enum FrontierDemand<'a> {
    Diagnostic,
    Observations { ready: &'a ReadinessProof },
}
/// What one occurrence's frontiers are prepared from: crate-private, as the preparation is.
pub(crate) struct FrontierRequest<'a> {
    pub(crate) demand: FrontierDemand<'a>,
    pub(crate) scope: &'a PhaseScope,
    pub(crate) rule: RuleIndex,
    pub(crate) first: ryo_values::domain::stored_facts::types::Bucket,
    pub(crate) facts: &'a dyn FactsFinish,
    pub(crate) project: &'a dyn FactsWorker,
    pub(crate) project_symbols: &'a ryo_language::domain::symbols::types::Symbols,
}
/// The place of one declared occurrence among the occurrences of its phase, counted from zero in declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OccurrenceNumber(pub usize);
/// One declared occurrence of a project phase: its rule, the inputs bound to it, the publication slot it reserved and what its
/// frontiers are prepared for. An ordinary occurrence binds nothing, reserves no slot and prepares its frontiers for the diagnostic.
#[derive(Clone, Copy)]
pub(crate) struct PhaseOccurrence<'a> {
    /// The occurrence's rule; a rule may occur more than once.
    pub rule: RuleIndex,
    /// The inputs bound to the occurrence.
    pub bound: &'a [BoundInput],
    /// The publication slot the occurrence reserved.
    pub slot: PublicationSlot,
    /// What the occurrence's frontiers are prepared for.
    pub demand: FrontierDemand<'a>,
}
impl std::fmt::Debug for PhaseOccurrence<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
/// What one project phase reads: its declared occurrences, the corpus they are scoped over and the facts their frontiers are read
/// from.
pub(crate) struct PhaseOccurrences<'a> {
    /// Each declared occurrence, in declaration order.
    pub occurrences: &'a [PhaseOccurrence<'a>],
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
    /// The files each rule was refused on, by rule.
    pub refused: &'a [RefusedFiles],
    /// Finishes the facts the frontiers are read from.
    pub facts: &'a dyn FactsFinish,
    /// The project's worker over the facts.
    pub project: &'a dyn FactsWorker,
    /// The symbols the project facts name.
    pub project_symbols: &'a Symbols,
    /// The arenas of the invocation.
    pub arenas: &'a [ArenaBytes],
}
impl std::fmt::Debug for PhaseOccurrences<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
/// What one project phase evaluates: one occurrence of each rule given, in order, the corpus they are scoped over and the facts
/// their frontiers are read from.
///
/// # Examples
///
/// ```
/// // It borrows a run's corpus, coverage and facts worker, which no public constructor builds: the field's type, checked.
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhaseRules;
/// fn rules<'a>(phase: &PhaseRules<'a>) -> &'a [RuleIndex] {
///     phase.rules
/// }
/// ```
pub struct PhaseRules<'a> {
    /// The rule of each occurrence, in declaration order; a rule may occur more than once.
    pub rules: &'a [RuleIndex],
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
    /// The files each rule was refused on, by rule.
    pub refused: &'a [RefusedFiles],
    /// Finishes the facts the frontiers are read from.
    pub facts: &'a dyn FactsFinish,
    /// The project's worker over the facts.
    pub project: &'a dyn FactsWorker,
    /// The symbols the project facts name.
    pub project_symbols: &'a Symbols,
    /// The arenas of the invocation.
    pub arenas: &'a [ArenaBytes],
}
impl std::fmt::Debug for PhaseRules<'_> {
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
/// Why one occurrence of a project phase has no completed outcome.
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
    /// The occurrence's frontier readiness, publication slot or bound inputs were refused before it reached the phase.
    Unbound(OperatingError),
}
/// One occurrence of a project phase once prepared: its scope, its frontiers, the inputs bound to it and its publication slot.
pub(crate) struct PreparedOccurrence<'a> {
    pub(crate) scope: PhaseScope,
    pub(crate) frontier: PreparedFrontiers,
    pub(crate) bound: &'a [BoundInput],
    pub(crate) slot: PublicationSlot,
}
/// One prepared occurrence once the caller gave its partials, its frontiers handed on to its task.
pub(crate) struct ReadyOccurrence<'a> {
    pub(crate) scope: PhaseScope,
    pub(crate) partials: PhasePartials,
    pub(crate) bound: &'a [BoundInput],
    pub(crate) slot: PublicationSlot,
}
