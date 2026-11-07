//! One project phase: its scope, its inputs and its tasks.
use crate::application::file_dag::frontiers::types::BoundInput;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::domain::coverage::types::Partials;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::rule_intent::types::RuleDemand;
/// One project-phase publication/interpreter and symbol generation, shared across rules.
pub struct ProjectPhase<'a> {
    pub(in crate::application::file_dag::program) request: &'a RequestExecution,
    pub(in crate::application::file_dag::program) publication:
        std::rc::Rc<crate::application::file_dag::program::facade_runtime::types::PublicationScope>,
}
pub struct ProjectScope {
    pub(in crate::application::file_dag::program) required: crate::application::file_dag::program::admission::RequiredPartitions,
    pub(in crate::application::file_dag::program) publication:
        std::rc::Rc<crate::application::file_dag::program::facade_runtime::types::PublicationScope>,
}
pub struct ProjectInput<'a> {
    pub scope: &'a ProjectScope,
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
