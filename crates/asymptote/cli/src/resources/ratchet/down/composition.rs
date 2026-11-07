//! The wiring of `down`: the edge chooses each port's adapter and builds the service it calls. It only constructs.
//! Nothing here implements a port.

use std::sync::Arc;

use asymptote_core::application::accepted_state::service::AcceptedState;
use asymptote_core::application::cached_report::service::CachedReport;
use asymptote_core::domain::manifest::service::reach;
use asymptote_core::domain::manifest::types::{ProductRoot, Reach};
use asymptote_core::inbound::use_cases::down::interface::DownUseCase;
use asymptote_core::outbound::manifest::errors::ManifestError;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct RatchetDownParts<S> {
    /// The service.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: RatchetDownParts<S>) -> S
where
    S: DownUseCase,
{
    parts.service
}

/// Selects the production products: the ones `check` reaches from `start`, the nearest `asymptote.toml`'s or a
/// workspace root's own and each member's, each over TOML, the disk, BLAKE3, its own check store, git and the system
/// clock.
///
/// # Errors
///
/// The manifest's, when no product is found, a manifest cannot be read, or a member cannot be one.
pub(crate) fn build(
    start: &ProductRoot,
    connections: &super::connections::RatchetDownConnections,
) -> Result<Reach<Box<dyn DownUseCase>>, ManifestError> {
    use asymptote_core::outbound::manifest::interface::ManifestPort as _;

    let manifest = asymptote_core::outbound::manifest::factory::toml();
    let root = manifest.find(start)?;
    let members = manifest.members(&root)?.0;
    // The root's own tools are read only when it is a workspace root: a product alone is reached whatever it holds.
    let tools = if members.is_empty() {
        Vec::new()
    } else {
        manifest.read(&root)?.tools
    };
    Ok(reach(
        &root,
        members,
        &tools,
        |root| -> Box<dyn DownUseCase> { Box::new(build_product(root, connections)) },
    ))
}

/// The service of the product at `root`.
fn build_product(
    root: &ProductRoot,
    connections: &super::connections::RatchetDownConnections,
) -> impl DownUseCase + use<> {
    use asymptote_core::inbound::use_cases::down::use_case::{DownPorts, DownUseCaseImpl};
    use asymptote_core::outbound::lock::interface::LockPort;
    use asymptote_core::outbound::manifest::interface::ManifestPort;
    use asymptote_core::outbound::repository::interface::RepositoryPort;
    use asymptote_core::outbound::{check_store, clock, lock, manifest, repository, sources};

    let lock: Arc<dyn LockPort> = Arc::new(lock::factory::toml(root.clone()));
    let repository: Arc<dyn RepositoryPort> = Arc::new(repository::factory::git(
        root.clone(),
        repository::factory::git(root.clone(), Ok(repository::types::Worktree::default()))
            .worktree(),
    ));
    let manifest: Arc<dyn ManifestPort> = Arc::new(manifest::factory::toml());
    compose(RatchetDownParts {
        service: DownUseCaseImpl::new(DownPorts {
            accepted: AcceptedState::new(Arc::clone(&lock), Arc::clone(&repository)),
            report: CachedReport::new(Arc::new(sources::factory::disk(root.clone()))),
            manifest,
            lock,
            store: Arc::new(check_store::factory::postcard(super::connections::store(
                connections,
                root,
            ))),
            repository,
            clock: Arc::new(clock::factory::system()),
        }),
    })
}
