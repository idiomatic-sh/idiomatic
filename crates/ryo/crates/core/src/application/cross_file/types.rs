//! Whether this phase owns a coordinator-reserved publication arena.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PublicationSlot {
    /// No named roots are demanded from this phase.
    Unused,
    /// An ordinary, unique index reserved by the existing request owner.
    Reserved(ryo_relations::domain::relations::types::WorkerNumber),
}

/// Required resources for the existing cross ready batches; no optional owner/fallback executor.
pub(crate) struct ObservationRun<'a> {
    pub domain: crate::domain::crossing::types::CrossingInput<'a>,
    pub executor: &'a dyn crate::application::file_dag::executor::PrimitiveExecutor,
    pub operators: std::sync::Arc<crate::domain::operator::contract::types::RequestOperators>,
    pub scheduler: &'a dyn crate::outbound::scheduler::interface::SchedulerPort,
    pub workers: crate::outbound::scheduler::types::WorkerCount,
    pub observations: crate::domain::file_program::types::ObservationRequest<'a>,
    pub first_bucket: &'a [ryo_values::domain::stored_facts::types::Bucket],
    pub facts: &'a dyn crate::outbound::facts::interface::FactsFinish,
    pub project: &'a dyn crate::outbound::facts::interface::FactsWorker,
    pub project_symbols: &'a ryo_language::domain::symbols::types::Symbols,
    pub todo: &'a [crate::domain::answering::types::Todo],
}

/// Admission preserves independent input successes and explicit refusal causes.
pub(crate) struct PreparedFrontiers {
    pub rows: Vec<crate::domain::table_model::types::Frontier>,
    pub refused: Vec<crate::domain::crossing::types::RefusedFrontier>,
}
