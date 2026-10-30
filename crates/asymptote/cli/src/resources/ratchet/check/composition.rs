//! The wiring of `check`: the edge chooses each port's adapter and builds the service it calls. It only constructs.
//! Nothing here implements a port.

use std::sync::Arc;

use asymptote_core::application::accepted_state::service::AcceptedState;
use asymptote_core::domain::manifest::service::reach;
use asymptote_core::domain::manifest::types::{ProductRoot, Reach};
use asymptote_core::inbound::use_cases::check::interface::CheckUseCase;
use asymptote_core::outbound::manifest::errors::ManifestError;
use asymptote_core::outbound::manifest::types::HookTable;

/// The dependencies the caller supplies: the products the command calls.
pub(crate) struct RatchetCheckParts<S> {
    /// The products.
    pub service: S,
}

/// Wires the products from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: RatchetCheckParts<S>) -> S {
    parts.service
}

/// Selects the production products: for the one whose `asymptote.toml` is nearest, or for a workspace root's own
/// tools and each of its members, each over TOML, the subprocess runner, serde-sarif and the JSON reader, the disk,
/// BLAKE3, git and a store of its own.
///
/// # Errors
///
/// The manifest's, when no product is found, a manifest cannot be read, or a member cannot be one.
pub(crate) fn build(
    start: &ProductRoot,
    connections: &super::connections::RatchetCheckConnections,
    on_signal: fn(Box<dyn Fn() + Send>),
) -> Result<Reach<Box<dyn CheckUseCase>>, ManifestError> {
    use asymptote_core::outbound::manifest::interface::ManifestPort as _;

    let manifest = asymptote_core::outbound::manifest::factory::toml();
    let root = manifest.find(start)?;
    let members = manifest.members(&root)?.0;
    // The repository is listed once, for every product in it.
    let worktree = {
        use asymptote_core::outbound::repository::interface::RepositoryPort as _;
        asymptote_core::outbound::repository::factory::git(
            root.clone(),
            Ok(asymptote_core::outbound::repository::types::Worktree::default()),
        )
        .worktree()
    };
    // The root's own tools are read only when it is a workspace root: a product alone is reached whatever it holds.
    let tools = if members.is_empty() {
        Vec::new()
    } else {
        manifest.read(&root)?.tools
    };
    Ok(compose(RatchetCheckParts {
        service: reach(&root, members, &tools, |root| {
            build_product(root.clone(), connections, &worktree, on_signal)
        }),
    }))
}

/// The product at `root`, over the production ports and a store of its own.
fn build_product(
    root: ProductRoot,
    connections: &super::connections::RatchetCheckConnections,
    worktree: &Result<
        asymptote_core::outbound::repository::types::Worktree,
        asymptote_core::outbound::repository::errors::RepositoryError,
    >,
    on_signal: fn(Box<dyn Fn() + Send>),
) -> Box<dyn CheckUseCase> {
    use asymptote_core::inbound::use_cases::check::types::CheckPorts;
    use asymptote_core::inbound::use_cases::check::use_case::CheckUseCaseImpl;
    use asymptote_core::outbound::{
        bmf, check_store, digest, lock, manifest, repository, sarif, sources, tools,
    };

    let running = tools::factory::ToolsConnection::new();
    let stopping = running.clone();
    on_signal(Box::new(move || stopping.stop()));
    Box::new(CheckUseCaseImpl::new(CheckPorts {
        manifest: Arc::new(manifest::factory::toml()),
        accepted: AcceptedState::new(
            Arc::new(lock::factory::toml(root.clone())),
            Arc::new(repository::factory::git(root.clone(), worktree.clone())),
        ),
        tools: Arc::new(tools::factory::subprocess(running)),
        sarif: Arc::new(sarif::factory::serde_sarif()),
        bmf: Arc::new(bmf::factory::json()),
        lock: Arc::new(lock::factory::toml(root.clone())),
        sources: Arc::new(sources::factory::disk(root.clone())),
        digest: Arc::new(digest::factory::blake3()),
        store: Arc::new(check_store::factory::postcard(super::connections::store(
            connections,
            &root,
        ))),
        repository: Arc::new(repository::factory::git(root, worktree.clone())),
    }))
}

/// The SARIF writer the edge renders a product's log with, when `--format sarif` asks for one.
pub(crate) fn build_sarif() -> impl asymptote_core::outbound::sarif::interface::SarifPort {
    asymptote_core::outbound::sarif::factory::serde_sarif()
}

/// The groups the tools of `workspace`'s products declare.
///
/// # Errors
///
/// The manifest's, when a product's cannot be read.
pub(crate) fn build_declared(
    workspace: &Reach<Box<dyn CheckUseCase>>,
) -> Result<Vec<asymptote_core::domain::manifest::types::Group>, ManifestError> {
    use asymptote_core::outbound::manifest::interface::ManifestPort as _;

    let manifest = asymptote_core::outbound::manifest::factory::toml();
    let mut groups = Vec::new();
    for product in &workspace.products {
        groups.extend(
            manifest
                .read(&product.root)?
                .tools
                .into_iter()
                .map(|tool| tool.group),
        );
    }
    Ok(groups)
}

/// The hooks the workspace root of `start` maps to groups.
///
/// # Errors
///
/// The manifest's, when the root cannot be found or read, or a hook maps a group no tool declares.
pub(crate) fn build_hooks(start: &ProductRoot) -> Result<HookTable, ManifestError> {
    use asymptote_core::outbound::manifest::interface::ManifestPort as _;

    asymptote_core::outbound::manifest::factory::toml().hooks(start)
}
