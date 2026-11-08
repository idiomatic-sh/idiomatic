//! The test support of the `execution` use case: the scenario its tests share.
use ryo_execution_store::outbound::derived::interface::DerivedPort;
use crate::outbound::operators::interface::OperatorsPort;
use std::sync::Arc;

#[cfg(test)]
pub(crate) mod requests;

/// The real operators capability, as the engine composes it.
#[must_use]
pub fn operators() -> impl OperatorsPort {
    crate::outbound::operators::factory::use_case(
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
            ryo_operators::outbound::matching::factory::use_case(),
        )),
    )
}
/// The derived capability behind its port, as the edge composes it.
#[must_use]
pub fn derived() -> impl DerivedPort {
    ryo_execution_store::outbound::derived::factory::semantics()
}
