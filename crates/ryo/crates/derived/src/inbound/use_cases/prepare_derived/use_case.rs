//! The `prepare` use case: stateless; concrete construction belongs at a composition edge.
use super::interface::PrepareDerivedUseCase;
use crate::domain::contract::types::InputSchemas;
use crate::domain::contract::types::prepared_derived::value::PreparedDerived;
use crate::domain::request::types::DerivedRequest;

/// `prepare`, over no port: it holds no state.
#[derive(Clone, Copy, Debug)]
pub struct PrepareDerivedUseCaseImpl;

impl PrepareDerivedUseCaseImpl {
    /// `prepare`, which holds nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PrepareDerivedUseCase for PrepareDerivedUseCaseImpl {
    fn prepare_derived(&self, request: DerivedRequest, schemas: InputSchemas) -> PreparedDerived {
        crate::domain::preparation::service::prepare(request, schemas.0)
    }
}
