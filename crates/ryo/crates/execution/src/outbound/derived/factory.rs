//! Builds the derived adapters, one function each. Every dependency arrives as a parameter from the edge that composes. A factory
//! builds nothing else, chooses nothing, and is called only by an edge.
use crate::outbound::derived::adapters::use_case::adapter::DerivedUseCaseAdapter;
use crate::outbound::derived::adapters::use_case::types::DerivedUseCases;
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::inbound::use_cases::{
    build::use_case::BuildUseCaseImpl, prepare::use_case::PrepareUseCaseImpl,
    view::use_case::ViewUseCaseImpl,
};

/// The adapter over the portable implementations of the three derived use cases.
#[must_use]
pub fn semantics() -> impl DerivedPort {
    DerivedUseCaseAdapter::new(DerivedUseCases {
        prepare: PrepareUseCaseImpl::new(),
        build: BuildUseCaseImpl::new(),
        view: ViewUseCaseImpl::new(),
    })
}

/// The port over the given implementation.
#[must_use]
pub fn use_case(semantics: impl DerivedPort) -> impl DerivedPort {
    semantics
}
