//! The `prepare` use case: stateless; concrete construction belongs at a composition edge.
use super::interface::PrepareUseCase;
use crate::domain::contract::types::prepared_derived::value::PreparedDerived;
use crate::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;

/// `prepare`, over no port: it holds no state.
#[derive(Clone, Copy, Debug)]
pub struct PrepareUseCaseImpl;

impl PrepareUseCaseImpl {
    /// `prepare`, which holds nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PrepareUseCase for PrepareUseCaseImpl {
    fn prepare(&self, request: DerivedRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived {
        crate::domain::preparation::service::prepare(request, schemas)
    }
}
