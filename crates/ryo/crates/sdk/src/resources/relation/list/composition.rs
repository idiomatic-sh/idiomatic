//! The wiring of `relation list`: the edge takes the parts its caller supplies, chooses each
//! implementation, and builds the service it calls. It only constructs: the verb opens no client, so it
//! has no connections. Nothing here implements a port.

use std::sync::Arc;

use ryo_core::inbound::use_cases::list_relations::interface::ListRelationsUseCase;
use ryo_core::outbound::language;

/// The dependencies the caller supplies: how the service is built.
pub(crate) struct RelationListParts<Construct> {
    /// Builds the service.
    pub service: Construct,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<Service, Construct>(parts: RelationListParts<Construct>) -> Service
where
    Service: ListRelationsUseCase,
    Construct: FnOnce() -> Service,
{
    (parts.service)()
}

/// Chooses the service. The catalog is ryo's own, so no language is linked.
pub(crate) fn build() -> impl ListRelationsUseCase {
    compose(RelationListParts {
        service: || {
            ryo_core::inbound::use_cases::list_relations::use_case::ListRelationsUseCaseImpl::new(
                Arc::new(language::factory::use_case(Vec::new())),
            )
        },
    })
}
