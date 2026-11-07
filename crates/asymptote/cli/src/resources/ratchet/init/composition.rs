//! The wiring of `init`: the edge chooses each port's adapter and builds the service it calls. It only constructs.
//! Nothing here implements a port.

use std::sync::Arc;

use asymptote_core::domain::manifest::types::ProductRoot;
use asymptote_core::inbound::use_cases::init::interface::InitUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct RatchetInitParts<S> {
    /// The service.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: RatchetInitParts<S>) -> S
where
    S: InitUseCase,
{
    parts.service
}

/// Selects the production service: TOML, the lock's files, and the hooks git keeps. The lock is the one `init`
/// writes where it writes a configuration: at the top level of the repository `start` is in. Where git cannot say
/// which that is, `init` refuses before it writes anything, so the lock is bound to `start`, never written.
pub(crate) fn build(start: &ProductRoot) -> impl InitUseCase + use<> {
    use asymptote_core::inbound::use_cases::init::types::InitPorts;
    use asymptote_core::inbound::use_cases::init::use_case::InitUseCaseImpl;
    use asymptote_core::outbound::hooks::interface::HooksPort as _;
    use asymptote_core::outbound::{hooks, lock, manifest};

    let hooks = hooks::factory::git();
    let top = hooks.top(start).unwrap_or_else(|_| start.clone());
    compose(RatchetInitParts {
        service: InitUseCaseImpl::new(InitPorts {
            manifest: Arc::new(manifest::factory::toml()),
            lock: Arc::new(lock::factory::toml(top)),
            hooks: Arc::new(hooks),
        }),
    })
}
