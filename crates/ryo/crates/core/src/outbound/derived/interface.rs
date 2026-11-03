//! The engine owns acquisition/lifetime; this port supplies finite semantic operations.
use ryo_derived::domain::contract::types::*;
pub trait DerivedPort: Send + Sync + std::fmt::Debug {
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
