//! The wiring of `operator describe`: the operators are the core's own, so the service calls no port
//! and the verb opens no client. Nothing here implements a port.

use ryo_core::inbound::use_cases::describe_operator::interface::DescribeOperatorUseCase;

/// The dependencies the caller supplies: how the service is built.
pub(crate) struct OperatorDescribeParts<Construct> {
    /// Builds the service.
    pub service: Construct,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<Service, Construct>(parts: OperatorDescribeParts<Construct>) -> Service
where
    Service: DescribeOperatorUseCase,
    Construct: FnOnce() -> Service,
{
    (parts.service)()
}

/// Chooses the service: the operators are the core's own, so it calls no port.
pub(crate) fn build() -> impl DescribeOperatorUseCase {
    compose(OperatorDescribeParts {
        service: || {
            ryo_core::inbound::use_cases::describe_operator::use_case::DescribeOperatorUseCaseImpl::new()
        },
    })
}
