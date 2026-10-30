//! The wiring of `relation describe`: the edge takes the parts its caller supplies and
//! builds the service it calls. It only constructs. Nothing here implements a port.

use ryo_core::inbound::use_cases::describe_relation::interface::DescribeRelationUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct RelationDescribeParts<S> {
    /// The service the command calls.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: RelationDescribeParts<S>) -> S
where
    S: DescribeRelationUseCase,
{
    parts.service
}

/// Selects the production service. The catalog is ryo's own; the languages linked say whether they read each relation.
pub(crate) fn build() -> impl DescribeRelationUseCase {
    use std::sync::Arc;

    use ryo_core::inbound::use_cases::describe_relation::use_case::DescribeRelationUseCaseImpl;

    compose(RelationDescribeParts {
        service: DescribeRelationUseCaseImpl::new(Arc::new(crate::languages::linked())),
    })
}
