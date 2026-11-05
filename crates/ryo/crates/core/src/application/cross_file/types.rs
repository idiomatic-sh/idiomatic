/// Diagnostic result plus the actual admitted identities of its complete named roots.
pub(crate) struct CrossedWithPublications {
    pub crossed: crate::domain::answering::types::Crossed,
    pub publications: Vec<(
        ryo_values::domain::observations::types::ObservationIndex,
        crate::application::file_dag::program::facade::ObservedPublication,
    )>,
}

/// Required resources for the existing cross ready batches; no optional owner/fallback executor.
pub(crate) struct ObservationRun<'a> {
    pub domain: crate::domain::crossing::types::CrossingInput<'a>,
    pub program: std::sync::Arc<crate::application::file_dag::program::facade::PreparedProgram>,
    pub services: &'a crate::application::file_dag::program::facade::ProgramServices<'a>,
    pub scheduler: &'a dyn crate::outbound::scheduler::interface::SchedulerPort,
    pub workers: crate::outbound::scheduler::types::WorkerCount,
    pub observations: crate::domain::file_program::types::ObservationRequest<'a>,
    pub first_bucket: &'a [ryo_values::domain::stored_facts::types::Bucket],
    pub facts: &'a dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsFinish,
    pub project: &'a dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker,
    pub project_symbols: &'a ryo_language::domain::symbols::types::Symbols,
    pub todo: &'a [crate::domain::answering::types::Todo],
}
