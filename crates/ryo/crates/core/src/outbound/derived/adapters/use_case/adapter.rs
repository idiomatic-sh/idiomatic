//! Constructed at a composition edge, with no row-level dependency construction.
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::*;
use ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase;
#[derive(Debug)]
pub(crate) struct DerivedUseCaseAdapter<T> {
    semantics: T,
}
impl<T> DerivedUseCaseAdapter<T> {
    pub(crate) const fn new(semantics: T) -> Self {
        Self { semantics }
    }
}
impl<T: DerivedUseCase + Send + Sync + std::fmt::Debug> DerivedPort for DerivedUseCaseAdapter<T> {
    fn prepare(
        &self,
        request: ViewRequest,
        schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    ) -> std::sync::Arc<PreparedDerived> {
        self.semantics.prepare(request, schemas)
    }
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: DerivedInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault> {
        self.semantics.build(prepared, inputs, room, recycled)
    }
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        self.semantics.view(prepared, payload)
    }
}
