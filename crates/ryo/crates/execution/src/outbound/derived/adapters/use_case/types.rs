//! The three derived use cases the adapter calls, which the edge built.
use ryo_derived::inbound::use_cases::build::interface::BuildUseCase;
use ryo_derived::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_derived::inbound::use_cases::view::interface::ViewUseCase;
use std::sync::Arc;

/// The three derived use cases, composed once at an edge and injected.
pub(crate) struct DerivedUseCases {
    /// The preparation of requests.
    pub prepare: Arc<dyn PrepareUseCase + Send + Sync>,
    /// The build of payloads.
    pub build: Arc<dyn BuildUseCase + Send + Sync>,
    /// The view of payloads.
    pub view: Arc<dyn ViewUseCase + Send + Sync>,
}

impl std::fmt::Debug for DerivedUseCases {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct(stringify!(DerivedUseCases)).finish()
    }
}
