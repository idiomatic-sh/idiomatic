//! Stateless semantic implementation; concrete construction belongs at a composition edge.
use super::interface::DerivedUseCase;
use crate::domain::contract::types::*;
#[derive(Clone, Copy, Debug)]
pub struct DerivedUseCaseImpl;
impl DerivedUseCaseImpl {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
impl DerivedUseCase for DerivedUseCaseImpl {
    fn prepare(
        &self,
        request: ViewRequest,
        schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    ) -> std::sync::Arc<PreparedDerived> {
        std::sync::Arc::new(crate::domain::preparation::service::prepare(request, schemas))
    }
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: DerivedInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault> {
        crate::domain::dispatch::service::build(prepared, inputs, room, recycled)
    }
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        crate::domain::dispatch::service::view(prepared, payload)
    }
}
