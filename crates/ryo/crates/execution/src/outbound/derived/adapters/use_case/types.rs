//! The portable implementations of the three derived use cases the adapter calls.
use ryo_derived::inbound::use_cases::build::use_case::BuildUseCaseImpl;
use ryo_derived::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_derived::inbound::use_cases::view::use_case::ViewUseCaseImpl;

/// The three derived use cases, composed once at an edge.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DerivedUseCases {
    /// The preparation of requests.
    pub prepare: PrepareUseCaseImpl,
    /// The build of payloads.
    pub build: BuildUseCaseImpl,
    /// The view of payloads.
    pub view: ViewUseCaseImpl,
}
