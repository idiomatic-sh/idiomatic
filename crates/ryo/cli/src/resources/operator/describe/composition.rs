//! The wiring of `operator describe`: the edge takes the parts its caller supplies and
//! builds the service it calls. It only constructs. Nothing here implements a port.

use ryo_core::inbound::use_cases::describe_operator::interface::DescribeOperatorUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct OperatorDescribeParts<S> {
    /// The service the command calls.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: OperatorDescribeParts<S>) -> S
where
    S: DescribeOperatorUseCase,
{
    parts.service
}

/// Selects the production service: the operators are the core's own, so it calls no port.
pub(crate) fn build() -> impl DescribeOperatorUseCase {
    use ryo_core::inbound::use_cases::describe_operator::use_case::DescribeOperatorUseCaseImpl;
    compose(OperatorDescribeParts {
        service: DescribeOperatorUseCaseImpl::new(std::sync::Arc::new(ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl)),
    })
}
