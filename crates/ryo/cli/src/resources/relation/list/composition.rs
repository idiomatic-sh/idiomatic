//! The wiring of `relation list`: the edge takes the parts its caller supplies and
//! builds the service it calls. It only constructs. Nothing here implements a port.

use ryo_core::inbound::use_cases::list_relations::interface::ListRelationsUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct RelationListParts<S> {
    /// The service the command calls.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: RelationListParts<S>) -> S
where
    S: ListRelationsUseCase,
{
    parts.service
}

/// Selects the production service. The catalog is ryo's own, so no language is linked.
pub(crate) fn build() -> impl ListRelationsUseCase {
    use std::sync::Arc;

    use ryo_core::inbound::use_cases::list_relations::use_case::ListRelationsUseCaseImpl;
    use ryo_core::outbound::language;

    compose(RelationListParts {
        service: ListRelationsUseCaseImpl::new(Arc::new(language::factory::use_case(Vec::new()))),
    })
}
