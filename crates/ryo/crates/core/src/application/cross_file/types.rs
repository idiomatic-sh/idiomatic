//! Whether this phase owns a coordinator-reserved publication arena.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PublicationSlot {
    /// No named roots are demanded from this phase.
    Unused,
    /// An ordinary, unique index reserved by the existing request owner.
    Reserved(ryo_relations::domain::relations::types::WorkerNumber),
}

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
    pub facts: &'a dyn crate::outbound::facts::interface::FactsFinish,
    pub project: &'a dyn crate::outbound::facts::interface::FactsWorker,
    pub project_symbols: &'a ryo_language::domain::symbols::types::Symbols,
    pub todo: &'a [crate::domain::answering::types::Todo],
}

/// Admission preserves independent input successes and explicit refusal causes.
pub(crate) struct PreparedFrontiers {
    pub rows: Vec<FrontierInput>,
    pub refused: Vec<crate::domain::crossing::types::RefusedFrontier>,
}

/// Admitted phase inputs. Compact sources have schemas/name mappings but no
/// decoded cell allocation; the same graph owner materializes them on demand.
pub(crate) enum FrontierInput {
    Decoded {
        frontier: crate::domain::table_model::types::Frontier,
        coverage: crate::application::file_dag::completion::CoverageState,
    },
    Compact {
        input: ryo_values::domain::plan_parts::types::Input,
        table: crate::domain::table_words::types::CompactTable,
        spelling: Vec<ryo_relations::domain::relations::types::Name>,
        coverage: crate::application::file_dag::completion::CoverageState,
    },
}
