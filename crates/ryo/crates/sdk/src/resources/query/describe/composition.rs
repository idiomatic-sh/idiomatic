//! The wiring of `query describe`: the edge chooses the syntax adapter whose grammar the service answers, and builds the
//! service it calls. It only constructs: the verb opens no client, so it has no connections. Nothing here implements a port.

use std::sync::Arc;

use ryo_query_language::inbound::use_cases::describe::interface::DescribeUseCase;

/// The dependencies the caller supplies: how the service is built over the adapter its builder chose.
pub(crate) struct QueryDescribeParts<Construct> {
    /// Builds the service.
    pub service: Construct,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<Service, Construct>(parts: QueryDescribeParts<Construct>) -> Service
where
    Service: DescribeUseCase,
    Construct: FnOnce() -> Service,
{
    (parts.service)()
}

/// Chooses the production syntax adapter and hands `compose` the construction of the service over it.
pub(crate) fn build() -> impl DescribeUseCase {
    compose(QueryDescribeParts {
        service: || {
            let syntax = Arc::new(ryo_query_language::outbound::syntax::factory::winnow());
            ryo_query_language::inbound::use_cases::describe::use_case::DescribeUseCaseImpl::new(
                syntax,
            )
        },
    })
}
