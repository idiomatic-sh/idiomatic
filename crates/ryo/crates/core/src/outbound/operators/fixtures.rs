//! Test composition for the portable operator capability.
use super::{adapters::use_case::adapter::OperatorsUseCaseAdapter, interface::OperatorsPort};

pub(crate) const fn port() -> &'static dyn OperatorsPort {
    static ADAPTER: OperatorsUseCaseAdapter = OperatorsUseCaseAdapter::new();
    &ADAPTER
}
