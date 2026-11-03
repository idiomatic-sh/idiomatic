//! Test composition for the portable operator capability.
pub mod contract;
pub mod fake;
#[cfg(test)]
mod fake_test;
#[cfg(test)]
use super::{adapters::use_case::adapter::OperatorsUseCaseAdapter, interface::OperatorsPort};
#[cfg(test)]
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;

#[cfg(test)]
pub(crate) const fn port() -> &'static dyn OperatorsPort {
    static ADAPTER: OperatorsUseCaseAdapter<OperatorsUseCaseImpl> =
        OperatorsUseCaseAdapter::new(OperatorsUseCaseImpl::new());
    &ADAPTER
}
