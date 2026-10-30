//! The wiring of `create`: the edge chooses each port's adapter and builds the service it calls. It only constructs.
//! Nothing here implements a port.

use std::sync::Arc;

use tastemaker_core::inbound::use_cases::create::interface::CreateUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct WorkspaceCreateParts<S> {
    /// The service.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: WorkspaceCreateParts<S>) -> S
where
    S: CreateUseCase,
{
    parts.service
}

/// Selects the production service.
pub(crate) fn build(
    store: &tastemaker_core::domain::files::types::DirectoryName,
) -> impl CreateUseCase {
    use tastemaker_core::inbound::use_cases::create::use_case::CreateUseCaseImpl;
    use tastemaker_core::outbound::{manifest, packages, repository};

    compose(WorkspaceCreateParts {
        service: CreateUseCaseImpl::new(
            Arc::new(repository::factory::disk(
                crate::languages::skipped(),
                store.clone(),
            )),
            Arc::new(manifest::factory::toml()),
            Arc::new(packages::factory::path()),
        ),
    })
}

/// Selects the languages this build links, which say whether a skeleton's `extension` is one a language reads.
pub(crate) fn languages() -> ryo::linter::Languages {
    crate::languages::linked()
}
