//! The types of the `cross_file` operation: what its batches read and what they deliver.
use super::completion::types::CrossedWithPublications;
use super::errors::CrossFileError;
use crate::domain::answering::types::Todo;
use ryo_execution::application::file_dag::frontiers::types::BoundInput;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution::application::file_dag::frontiers::types::PreparedFrontiers;
use ryo_execution::application::file_dag::program::project_phase::types::PhaseScope;
use ryo_execution::application::file_dag::frontiers::types::PublicationSlot;
use ryo_execution::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::outbound::facts::interface::FactsFinish;
use ryo_execution::outbound::facts::interface::FactsWorker;
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
