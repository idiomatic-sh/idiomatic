//! The wiring of `check`: the edge chooses each port's adapter and builds the service it calls. It only constructs.
//! Nothing here implements a port.

use std::sync::Arc;

use tastemaker_core::inbound::use_cases::check::interface::CheckUseCase;
use tastemaker_core::inbound::use_cases::validate::interface::ValidateUseCase;

/// The dependencies the caller supplies: the service the command calls, and the `validate` it proves with first.
pub(crate) struct WorkspaceCheckParts<S, V> {
    /// The service.
    pub service: S,
    /// What proves every definition before the check.
    pub validate: V,
}

/// Wires the services from the supplied dependencies; it only constructs.
pub(crate) fn compose<S, V>(parts: WorkspaceCheckParts<S, V>) -> (S, V)
where
    S: CheckUseCase,
    V: ValidateUseCase,
{
    (parts.service, parts.validate)
}

/// Selects the production service over the stores `connections` opened: the disk, TOML, path packages, ryo over the
/// languages this build links, the first keeping its cache in the workspace's unless under `-n/--dry-run`, the postcard store of
/// `build`, blake3, globset and minijinja; and `validate` over the same adapters, wired as `validate` wires it.
pub(crate) fn build(
    store: &tastemaker_values::domain::files::types::DirectoryName,
    connections: &super::connections::WorkspaceCheckConnections,
) -> (impl CheckUseCase + use<>, impl ValidateUseCase + use<>) {
    use tastemaker_core::inbound::use_cases::check::use_case::{CheckPorts, CheckUseCaseImpl};
    use tastemaker_core::inbound::use_cases::validate::use_case::{
        ValidatePorts, ValidateUseCaseImpl,
    };
    use tastemaker_core::outbound::{
        analysis, check_store, digest, manifest, matching, packages, repository, templates,
        validate_store,
    };

    let repository: Arc<dyn repository::interface::RepositoryPort> = Arc::new(
        repository::factory::disk(crate::languages::skipped(), store.clone()),
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
    // ONE analysis adapter, one engine, serves the check's lint, the lint that judges the counts and pairs, and each scenario's: the
    // plans it compiled and the globs it made serve every call after, and the run holds one engine in memory. It keeps nothing in ryo's
    // session store, as the fast composition's plain mode does: Tastemaker's own store is what makes a rerun of an unchanged project fast.
    let analysis: Arc<dyn analysis::interface::AnalysisPort> = Arc::new(analysis::factory::ryo(
        crate::languages::linked(),
        analysis::types::AnalysisSettings {
            pool,
            memory: analysis::types::Memory::Machine,
        },
    ));
    let counting = Arc::clone(&analysis);
    let scenarios = Arc::clone(&analysis);
    let reporting = Arc::new(
        tastemaker_core::application::reporting::service::ReportingService::new(
            Arc::clone(&scheduler),
            Arc::clone(&matching),
            Arc::clone(&digest),
        ),
    );
    // Each scenario's project is written in memory, and the report of its check reads it there.
    let fixtures: Arc<dyn repository::interface::RepositoryPort> = Arc::new(
        repository::factory::memory(crate::languages::skipped(), store.clone()),
    );
    let fixture_reporting = Arc::new(
        tastemaker_core::application::reporting::service::ReportingService::new(
            Arc::clone(&scheduler),
            Arc::clone(&matching),
            Arc::clone(&digest),
        ),
    );
    // One proofs store for the validation and the check that consults it: under `-n/--dry-run` it keeps the proofs in
    // memory only, and the check reads there what the validation just proved.
    let proofs: Arc<dyn validate_store::interface::ValidateStorePort> =
        Arc::new(validate_store::factory::postcard(&connections.store));
    compose(WorkspaceCheckParts {
        validate: ValidateUseCaseImpl::new(ValidatePorts {
            scheduler: Arc::clone(&scheduler),
            reporting: fixture_reporting,
            repository: Arc::clone(&repository),
            fixtures,
            settings: connections.store.clone(),
            manifest: Arc::new(manifest::factory::toml()),
            packages: Arc::new(packages::factory::path()),
            analysis: scenarios,
            store: Arc::clone(&proofs),
            digest: Arc::clone(&digest),
            matching: Arc::clone(&matching),
            templates: Arc::new(templates::factory::minijinja()),
        }),
        service: CheckUseCaseImpl::new(CheckPorts {
            reporting,
            repository,
            manifest: Arc::new(manifest::factory::toml()),
            packages: Arc::new(packages::factory::path()),
            analysis,
            counting,
            store: Arc::new(check_store::factory::postcard(&connections.store)),
            proofs,
            digest,
            matching,
            templates: Arc::new(templates::factory::minijinja()),
        }),
    })
}
