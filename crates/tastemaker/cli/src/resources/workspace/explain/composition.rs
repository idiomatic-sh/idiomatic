//! The wiring of `explain`: the edge chooses each port's adapter and builds the service it calls. It only constructs.
//! Nothing here implements a port.

use std::sync::Arc;

use tastemaker_core::inbound::use_cases::explain::interface::ExplainUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct WorkspaceExplainParts<S> {
    /// The service.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: WorkspaceExplainParts<S>) -> S
where
    S: ExplainUseCase,
{
    parts.service
}

/// Selects the production service.
pub(crate) fn build(
    store: &tastemaker_core::domain::files::types::DirectoryName,
) -> impl ExplainUseCase {
    use tastemaker_core::inbound::use_cases::explain::use_case::ExplainUseCaseImpl;
    use tastemaker_core::outbound::{analysis, manifest, matching, packages, repository};

    compose(WorkspaceExplainParts {
        service: ExplainUseCaseImpl::new(
            Arc::new(repository::factory::disk(
                crate::languages::skipped(),
                store.clone(),
            )),
            Arc::new(manifest::factory::toml()),
            Arc::new(packages::factory::path()),
            Arc::new(matching::factory::globset()),
            Arc::new(analysis::factory::ryo(
                crate::languages::linked(),
                analysis::types::AnalysisSettings {
                    // A run of one small lint call at most: it keeps nothing, and reads on a pool of one worker, built here once.
                    pool: tastemaker_core::outbound::scheduler::factory::pool(
                        tastemaker_core::outbound::scheduler::types::WorkerCount(1),
                    ),
                    memory: analysis::types::Memory::Machine,
                },
            )),
        ),
    })
}
