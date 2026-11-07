//! The wiring of `operator list`: the edge takes the parts its caller supplies and
//! builds the service it calls. It only constructs. Nothing here implements a port.

use ryo_core::inbound::use_cases::list_operators::interface::ListOperatorsUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct OperatorListParts<S> {
    /// The service the command calls.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: OperatorListParts<S>) -> S
where
    S: ListOperatorsUseCase,
{
    parts.service
}

/// Selects the production service: the operators are the core's own, so it calls no port.
pub(crate) fn build() -> impl ListOperatorsUseCase {
    use ryo_core::inbound::use_cases::list_operators::use_case::ListOperatorsUseCaseImpl;
    compose(OperatorListParts {
        service: ListOperatorsUseCaseImpl,
    })
}
