//! Cohesive portable derived semantics; all inputs are finite borrowed data.
use crate::domain::contract::types::*;
pub trait DerivedUseCase: Sync {
    fn prepare(
        &self,
        request: ViewRequest,
        schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    ) -> std::sync::Arc<PreparedDerived>;
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: DerivedInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault>;
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault>;
}
