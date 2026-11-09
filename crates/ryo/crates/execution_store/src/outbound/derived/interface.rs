//! What the engine asks of the portable derived semantics: prepare a request against its input schemas, build the payload from
//! borrowed rows, and view a built payload; execution authority never crosses it.
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::contract::types::{
    BuildInputs, BuildOutcome, BuildWorkspace, DerivedView,
};
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;

/// What every `derived` adapter implements.
pub trait DerivedPort: Send + Sync + std::fmt::Debug {
    /// Checks a request against the schemas of its inputs; the answer is validated by the caller.
    fn prepare(&self, request: DerivedRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived;

    /// Builds the payload of a prepared request from borrowed rows, over a workspace and a payload to recycle.
    ///
    /// # Errors
    ///
    /// The portable fault the build refuses with.
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: BuildInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault>;

    /// A view over a built payload.
    ///
    /// # Errors
    ///
    /// The portable fault the view refuses with.
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault>;
}
