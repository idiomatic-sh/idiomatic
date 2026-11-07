//! The wiring of `list`: the edge chooses each port's adapter and builds the service it calls. It only constructs.
//! Nothing here implements a port.

use std::sync::Arc;

use tastemaker_core::inbound::use_cases::list::interface::ListUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct WorkspaceListParts<S> {
    /// The service.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: WorkspaceListParts<S>) -> S
where
    S: ListUseCase,
{
    parts.service
}

/// Selects the production service.
pub(crate) fn build(
    store: &tastemaker_core::domain::files::types::DirectoryName,
) -> impl ListUseCase {
    use tastemaker_core::inbound::use_cases::list::use_case::ListUseCaseImpl;
    use tastemaker_core::outbound::{analysis, manifest, packages, repository};

    compose(WorkspaceListParts {
        service: ListUseCaseImpl::new(
            Arc::new(repository::factory::disk(
                crate::languages::skipped(),
                store.clone(),
            )),
            Arc::new(manifest::factory::toml()),
            Arc::new(packages::factory::path()),
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
