//! One request execution: the state its files and project phase share.
use crate::application::file_dag::program::project_phase::types::ProjectScope;
use crate::domain::coverage::types::RuleIndex;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::interface::FactsWorker;
use std::sync::Arc;
pub struct RequestExecution {
    pub(in crate::application::file_dag::program) program:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    pub(in crate::application::file_dag::program) services:
        Arc<crate::application::file_dag::program::facade_runtime::types::ProgramServices>,
    /// Prepared frontiers of this request, one acquisition per population.
    pub(in crate::application::file_dag::program) frontiers: crate::application::file_dag::frontiers::types::FrontierCache,
}
pub enum FrontierDemand<'a> {
    Diagnostic,
    Observations { ready: &'a [bool] },
}
pub struct FrontierRequest<'a> {
    pub demand: FrontierDemand<'a>,
    pub scope: &'a ProjectScope,
    pub rule: RuleIndex,
    pub first: ryo_values::domain::stored_facts::types::Bucket,
    pub facts: &'a dyn FactsFinish,
    pub project: &'a dyn FactsWorker,
    pub project_symbols: &'a ryo_language::domain::symbols::types::Symbols,
}
