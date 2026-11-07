//! Composition edge for the portable implementation.
use crate::outbound::derived::interface::DerivedPort;
use crate::outbound::derived::types::DerivedUseCases;
use ryo_derived::inbound::use_cases::{
    build::use_case::BuildUseCaseImpl, prepare::use_case::PrepareUseCaseImpl,
    view::use_case::ViewUseCaseImpl,
};

/// The portable implementations of the three derived use cases.
#[must_use]
pub const fn semantics() -> DerivedUseCases {
    DerivedUseCases {
        prepare: PrepareUseCaseImpl::new(),
        build: BuildUseCaseImpl::new(),
        view: ViewUseCaseImpl::new(),
    }
}

/// The port over the given implementations.
#[must_use]
pub fn use_case(semantics: impl DerivedPort) -> impl DerivedPort {
    semantics
}
