/// Diagnostic result plus the actual admitted identities of its complete and partial named roots.
pub(crate) struct CrossedWithPublications {
    pub crossed: crate::domain::answering::types::Crossed,
    pub publications: Vec<(
        ryo_values::domain::observations::types::ObservationIndex,
        ryo_execution::inbound::use_cases::execution::types::ObservedPublication,
    )>,
}

/// Required resources for the existing cross ready batches; no optional owner/fallback executor.
pub(crate) struct ObservationRun<'a> {
    pub domain: crate::domain::crossing::types::CrossingInput<'a>,
    pub program:
        std::sync::Arc<ryo_execution::inbound::use_cases::execution::types::PreparedProgram>,
    pub services: &'a ryo_execution::inbound::use_cases::execution::types::RequestExecution,
    pub batches: &'a [Vec<ryo_execution::inbound::use_cases::execution::types::RuleIndex>],
    pub first_bucket: &'a [ryo_values::domain::stored_facts::types::Bucket],
    pub facts: &'a dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsFinish,
    pub project: &'a dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker,
    pub project_symbols: &'a ryo_language::domain::symbols::types::Symbols,
    pub todo: &'a [crate::domain::answering::types::Todo],
}

/// One declared task; the scope and bindings belong to the shared project stage.
pub(crate) struct ProjectTask<'a> {
    pub todo: &'a crate::domain::answering::types::Todo,
    pub frontier: ryo_execution::inbound::use_cases::execution::types::PreparedFrontiers,
    pub bound: &'a [ryo_execution::inbound::use_cases::execution::types::BoundInput],
    pub slot: ryo_execution::inbound::use_cases::execution::types::PublicationSlot,
    pub scope: &'a ryo_execution::inbound::use_cases::execution::types::ProjectScope,
}
