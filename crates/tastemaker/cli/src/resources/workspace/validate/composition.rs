//! The wiring of `validate`: the edge chooses each port's adapter and builds the service it calls. It only constructs.
//! Nothing here implements a port.

use std::sync::Arc;

use tastemaker_core::inbound::use_cases::validate::interface::ValidateUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct WorkspaceValidateParts<S> {
    /// The service.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: WorkspaceValidateParts<S>) -> S
where
    S: ValidateUseCase,
{
    parts.service
}

/// Selects the production service over the stores `connections` opened, its store `build`'s.
pub(crate) fn build(
    store: &tastemaker_core::domain::files::types::DirectoryName,
    connections: &super::connections::WorkspaceValidateConnections,
) -> impl ValidateUseCase + use<> {
    use tastemaker_core::inbound::use_cases::validate::use_case::{
        ValidatePorts, ValidateUseCaseImpl,
    };
    use tastemaker_core::outbound::{
        analysis, digest, manifest, matching, packages, repository, templates, validate_store,
    };

    let repository: Arc<dyn repository::interface::RepositoryPort> = Arc::new(
        repository::factory::disk(crate::languages::skipped(), store.clone()),
    );
    // Each scenario's project is written in memory, and the report of its check reads it there.
    let fixtures: Arc<dyn repository::interface::RepositoryPort> = Arc::new(
        repository::factory::memory(crate::languages::skipped(), store.clone()),
    );
    // The run's one pool, built once here and injected into every consumer: the scheduler and ryo's engine alike.
    let pool = tastemaker_core::outbound::scheduler::factory::pool(connections.workers);
    let scheduler: Arc<dyn tastemaker_core::outbound::scheduler::interface::SchedulerPort> =
        Arc::new(tastemaker_core::outbound::scheduler::factory::rayon(
            pool.clone(),
        ));
    let digest: Arc<dyn digest::interface::DigestPort> = Arc::new(digest::factory::blake3());
    let matching: Arc<dyn matching::interface::MatchingPort> =
        Arc::new(matching::factory::globset());
    let reporting = Arc::new(
        tastemaker_core::application::reporting::service::ReportingService::new(
            Arc::clone(&scheduler),
            Arc::clone(&matching),
            Arc::clone(&digest),
        ),
    );
    // A scenario's lint call keeps nothing.
    let analysis = analysis::factory::ryo(
        crate::languages::linked(),
        analysis::types::AnalysisSettings {
            pool,
            memory: analysis::types::Memory::Machine,
        },
    );
    compose(WorkspaceValidateParts {
        service: ValidateUseCaseImpl::new(ValidatePorts {
            scheduler,
            reporting,
            repository,
            fixtures,
            settings: connections.store.clone(),
            manifest: Arc::new(manifest::factory::toml()),
            packages: Arc::new(packages::factory::path()),
            analysis: Arc::new(analysis),
            store: Arc::new(validate_store::factory::postcard(&connections.store)),
            digest,
            matching,
            templates: Arc::new(templates::factory::minijinja()),
        }),
    })
}
