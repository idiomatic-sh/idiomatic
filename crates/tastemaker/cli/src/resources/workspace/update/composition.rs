//! The wiring of `update`: the edge chooses each port's adapter and builds the service it calls. It only constructs.
//! Nothing here implements a port.

use std::sync::Arc;

use tastemaker_core::inbound::use_cases::update::interface::UpdateUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct WorkspaceUpdateParts<S> {
    /// The service.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: WorkspaceUpdateParts<S>) -> S
where
    S: UpdateUseCase,
{
    parts.service
}

/// Selects the production service.
pub(crate) fn build(
    store: &tastemaker_core::domain::files::types::DirectoryName,
) -> impl UpdateUseCase {
    use tastemaker_core::inbound::use_cases::update::use_case::UpdateUseCaseImpl;
    use tastemaker_core::outbound::{manifest, matching, packages, repository};

    compose(WorkspaceUpdateParts {
        service: UpdateUseCaseImpl::new(
            Arc::new(repository::factory::disk(
                crate::languages::skipped(),
                store.clone(),
            )),
            Arc::new(manifest::factory::toml()),
            Arc::new(packages::factory::path()),
            Arc::new(matching::factory::globset()),
        ),
    })
}
