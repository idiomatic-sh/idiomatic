//! One project phase: its scope, its inputs and its tasks.
use crate::application::file_dag::frontiers::types::BoundInput;
use crate::application::file_dag::frontiers::types::InputReadiness;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_execution_domain::domain::coverage::types::Partials;
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
    Observations { ready: &'a [InputReadiness] },
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
