//! The types of the `cross_file` operation: what its batches read and what they deliver.
use super::completion::types::CrossedWithPublications;
use super::errors::CrossFileError;
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::BoundInput;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_execution::inbound::use_cases::execution::interface::PreparedFrontiers;
use ryo_execution::inbound::use_cases::execution::interface::PhaseScope;
use ryo_execution::inbound::use_cases::execution::interface::PublicationSlot;
use ryo_execution::inbound::use_cases::execution::interface::RequestExecution;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket};
use std::ops::Deref;
use std::sync::Arc;

/// What the cross-file steps of every rule read, with the program and the arenas of the invocation.
#[derive(Clone)]
pub(crate) struct ProjectCrossing<'a> {
    /// Invocation-owned physical arena lease view, supplied by the application coordinator.
    pub arenas: &'a [ArenaBytes],
    /// The one prepared executable program shared by all scopes in this request.
    pub program: Arc<PreparedExecutionProgram>,
    /// What the cross-file steps of every rule read besides the rows the files handed over.
    pub domain: crate::domain::crossing::types::CrossingInput<'a>,
}

impl<'a> Deref for ProjectCrossing<'a> {
    type Target = crate::domain::crossing::types::CrossingInput<'a>;
    fn deref(&self) -> &Self::Target {
        &self.domain
    }
}

/// Required resources for the existing cross ready batches; no optional owner/fallback executor.
pub(crate) struct ObservationRun<'a> {
    /// What the cross-file steps of every rule read.
    pub domain: crate::domain::crossing::types::CrossingInput<'a>,
    /// The one prepared executable program shared by all scopes in this request.
    pub program: Arc<PreparedExecutionProgram>,
    /// The request's execution capabilities.
    pub services: &'a RequestExecution,
    /// The rules of each ready batch, in order.
    pub batches: &'a [Vec<RuleIndex>],
    /// The first bucket of each rule's frontier.
    pub first_bucket: &'a [Bucket],
    /// Finishes the facts the frontiers are read from.
    pub facts: &'a dyn FactsFinish,
    /// The project's worker over the facts.
    pub project: &'a dyn FactsWorker,
    /// The symbols the project facts name.
    pub project_symbols: &'a Symbols,
    /// The occurrences of the declared rules to run.
    pub todo: &'a [Todo],
}

/// Each delivered occurrence retains the immutable declaration it accounts for.
pub(crate) struct ProjectOutcome {
    /// The declaration the outcome accounts for.
    pub todo: Todo,
    /// What the occurrence delivered, or why it failed.
    pub result: Result<CrossedWithPublications, CrossFileError>,
}

/// The resources one occurrence is evaluated from, once its preparation succeeded.
pub(crate) struct ReadyProject {
    /// The occurrence's project scope.
    pub scope: PhaseScope,
    /// The prepared frontiers, taken once when the task is built.
    pub frontier: Option<PreparedFrontiers>,
    /// The bound inputs of the occurrence.
    pub bound: Vec<BoundInput>,
    /// The publication slot the occurrence reserved.
    pub slot: PublicationSlot,
    /// The partials the diagnostic evaluation sees.
    pub evaluation: Partials,
    /// The partials the publication sees.
    pub publication: Partials,
}

/// What the ordinary project phase reads: the crossing, the request's execution, the rules to make and what their frontiers are read
/// from, and the budget each rule is made within.
pub(crate) struct OrdinaryRun<'a> {
    /// What the cross-file steps of every rule read, with the program and the arenas.
    pub crossing: ProjectCrossing<'a>,
    /// The request's execution capabilities.
    pub services: &'a RequestExecution,
    /// The occurrences of the ordinary rules to make, in order.
    pub todo: &'a [Todo],
    /// The first bucket of each rule's frontier.
    pub first_bucket: &'a [Bucket],
    /// Finishes the facts the frontiers are read from.
    pub facts: &'a dyn FactsFinish,
    /// The project's worker over the facts.
    pub project: &'a dyn FactsWorker,
    /// The symbols the project facts name.
    pub project_symbols: &'a Symbols,
    /// The rules' ids, by rule.
    pub ids: &'a [ryo_values::domain::runs::types::RuleId],
    /// The most memory the run may hold.
    pub budget: ryo_values::domain::runs::types::MemoryBytes,
}

/// What the ordinary project phase came to: each rule's outcome in order, the budget guard's refusal, the release of the stage and
/// the symbols the phase named.
pub(crate) struct OrdinaryOutcome {
    /// Each evaluated rule's outcome, in the order of the rules.
    pub outcomes: Vec<ProjectOutcome>,
    /// The budget guard's refusal, when it stopped the batch.
    pub refused: Option<CrossFileError>,
    /// The release of the shared stage.
    pub finished: Result<(), CrossFileError>,
    /// The symbols the phase named.
    pub symbols: Symbols,
}

/// What the cross-file phase of a run reads of it: what the store kept and what the run uses again of it, the rules, the files, the
/// request's program and execution, and the budget each rule is made within.
pub(crate) struct PhaseRun<'a> {
    /// What the run uses again of what the last kept.
    pub again: &'a ryo_values::domain::stores::types::Reuse,
    /// What the last run of this build kept.
    pub stored: &'a ryo_values::domain::stores::types::Stored,
    /// The rules' ids.
    pub ids: &'a [ryo_values::domain::runs::types::RuleId],
    /// The files whose stored rows are theirs no more.
    pub read_again: &'a [ryo_language::domain::files::types::FilePath],
    /// What the rules could not decide in the changed files read alone before the run.
    pub alone: &'a [ryo_execution::inbound::use_cases::metadata::interface::Unknown],
    /// Pure rule facts retained for source demand, frontier and work accounting.
    pub metadata: &'a [ryo_values::domain::run_metadata::types::RuleRunMetadata],
    /// What each rule needs of the files.
    pub needs: &'a [ryo_execution::inbound::use_cases::metadata::interface::Needs],
    /// Which rule governs which file.
    pub placed: &'a ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes,
    /// The path of each file, by file number.
    pub paths: &'a [ryo_language::domain::files::types::FilePath],
    /// The files, in the order of their paths.
    pub files: &'a [ryo_values::domain::files::types::CorpusInput],
    /// How the rows the rules emit are answered.
    pub answering: &'a crate::domain::render::types::Answering,
    /// The one prepared executable program shared by all scopes in this request.
    pub program: &'a Arc<PreparedExecutionProgram>,
    /// The request's execution capabilities.
    pub services: &'a RequestExecution,
    /// The first bucket of each rule's frontier.
    pub first_bucket: &'a [Bucket],
    /// The most memory the run may hold.
    pub budget: ryo_values::domain::runs::types::MemoryBytes,
}

/// The ports the cross-file phase finishes the facts with, names the languages' reasons by and is guarded by.
#[derive(Clone, Copy)]
pub(crate) struct PhasePorts<'a> {
    /// Finishes the words the files handed the cross-file steps.
    pub facts: &'a dyn ryo_execution_runtime::outbound::facts::interface::FactsPort,
    /// Names the reasons of the linked languages.
    pub language: &'a dyn crate::outbound::language::interface::LanguagePort,
    /// Says how much memory the process holds.
    pub memory: &'a dyn crate::outbound::memory::interface::MemoryPort,
}

/// What is left to settle of the cross-file phase: nothing, or the ordinary phase's outcomes.
pub(crate) enum Settling {
    /// The phase settled its answers itself, or made no rule.
    Settled,
    /// The ordinary phase's outcomes, for the caller to settle.
    Ordinary(OrdinaryOutcome),
}

/// What the cross-file phase came to: the project, and what is left to settle.
pub(crate) struct PhaseOutcome {
    /// The project made now, made again or kept.
    pub project: crate::domain::project::types::Projected,
    /// What is left to settle.
    pub settling: Settling,
}
