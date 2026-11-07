//! Production wiring for initialization.
use std::sync::Arc;
use tastemaker_core::inbound::use_cases::init::interface::InitUseCase;
/// The supplied initialization service.
pub(crate) struct WorkspaceInitParts<S> {
    /// The service called by the command.
    pub service: S,
}
/// Uses the supplied service.
pub(crate) fn compose<S>(parts: WorkspaceInitParts<S>) -> S
where
    S: InitUseCase,
{
    parts.service
}
/// Selects the filesystem repository.
pub(crate) fn build(
    store: &tastemaker_core::domain::files::types::DirectoryName,
) -> impl InitUseCase {
    compose(WorkspaceInitParts {
        service: tastemaker_core::inbound::use_cases::init::use_case::InitUseCaseImpl::new(
            Arc::new(tastemaker_core::outbound::repository::factory::disk(
                crate::languages::skipped(),
                store.clone(),
            )),
            Arc::new(tastemaker_core::outbound::manifest::factory::toml()),
        ),
    })
}
