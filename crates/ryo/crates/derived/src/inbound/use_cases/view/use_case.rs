//! The `view` use case: stateless; concrete construction belongs at a composition edge.
use super::interface::ViewUseCase;
use crate::domain::contract::types::DerivedView;
use crate::domain::contract::types::derived_payload::value::DerivedPayload;
use crate::domain::contract::types::prepared_derived::value::PreparedDerived;
use crate::domain::fault::types::DerivedFault;

/// `view`, over no port: it holds no state.
#[derive(Clone, Copy, Debug)]
pub struct ViewUseCaseImpl;

impl ViewUseCaseImpl {
    /// `view`, which holds nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl ViewUseCase for ViewUseCaseImpl {
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        crate::domain::dispatch::service::view(prepared, payload)
    }
}
