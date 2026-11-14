//! One project phase: what a caller hands it and what it answers.
use crate::application::file_dag::frontiers::errors::FrontiersError;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::interface::FactsWorker;
use ryo_execution_domain::domain::coverage::types::Covered;
use ryo_execution_domain::domain::coverage::types::InapplicableFiles;
use ryo_execution_domain::domain::coverage::types::Partials;
use ryo_execution_domain::domain::coverage::types::RefusedFiles;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::ArenaBytes;
/// The place of one declared occurrence among the occurrences of its phase, counted from zero in declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OccurrenceNumber(pub usize);
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
    Frontiers(FrontiersError),
    /// The engine answered fewer outcomes than prepared occurrences, with no refusal to explain it.
    Unevaluated,
    /// The occurrence's frontier readiness, publication slot or bound inputs were refused before it reached the phase.
    Unbound(OperatingError),
}
/// One project phase of a request: its occurrences run after the files through the project pass, in one symbol generation.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectPhase;
/// fn phase(phase: &ProjectPhase<'_>) {
///     let _ = phase;
/// }
/// ```
///
/// Only its crate opens one: no field is reachable outside it.
///
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectPhase;
/// fn forge(handle: &ProjectPhase<'_>) {
///     let _ = &handle.request;
/// }
/// ```
pub struct ProjectPhase<'a> {
    pub(crate) request:
        &'a crate::application::file_dag::program::request_execution::types::RequestExecution,
}
