//! The wiring of `operator list`: the operators are the core's own, so the service calls no port
//! and the verb opens no client. Nothing here implements a port.

use ryo_core::inbound::use_cases::list_operators::interface::ListOperatorsUseCase;

/// The dependencies the caller supplies: how the service is built.
pub(crate) struct OperatorListParts<Construct> {
    /// Builds the service.
    pub service: Construct,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<Service, Construct>(parts: OperatorListParts<Construct>) -> Service
where
    Service: ListOperatorsUseCase,
    Construct: FnOnce() -> Service,
{
    (parts.service)()
}

/// Chooses the service: the operators are the core's own, so it calls no port.
pub(crate) fn build() -> impl ListOperatorsUseCase {
    compose(OperatorListParts {
        service: || {
            ryo_core::inbound::use_cases::list_operators::use_case::ListOperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl))
        },
    })
}
