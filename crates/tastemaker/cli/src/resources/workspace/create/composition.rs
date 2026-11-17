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
    store: &tastemaker_values::domain::files::types::DirectoryName,
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

/// Whether a language this build links reads a scenario file ending in `extension`: a skeleton's scenarios in an
/// extension no linked language reads would find nothing.
pub(crate) fn reads_scenarios(extension: &str) -> bool {
    let scenario = ryo::linter::RelativePath(format!("scenario.{extension}").into());
    crate::languages::linked().reads(&scenario) != ryo::languages::Recognition::Declines
}
