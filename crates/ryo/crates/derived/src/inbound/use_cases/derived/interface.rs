//! Cohesive portable derived semantics; all inputs are finite borrowed data.
use crate::domain::contract::types::*;
/// The derived capability: three required methods, and a view that borrows a built payload.
///
/// A complete implementation compiles.
///
/// ```
/// use ryo_derived::domain::contract::types::*;
/// use ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase;
/// struct Item;
/// impl DerivedUseCase for Item {
///     fn prepare(&self, _request: ViewRequest, _schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>) -> std::sync::Arc<PreparedDerived> { unimplemented!() }
///     fn build(&self, _prepared: &PreparedDerived, _inputs: DerivedInputs<'_>, _room: &mut BuildWorkspace, _recycled: Option<DerivedPayload>) -> Result<BuildOutcome, DerivedFault> { unimplemented!() }
///     fn view<'a>(&self, _prepared: &PreparedDerived, _payload: &'a DerivedPayload) -> Result<DerivedView<'a>, DerivedFault> { unimplemented!() }
/// }
/// ```
///
/// A missing `prepare` is refused.
///
/// ```compile_fail,E0046
/// use ryo_derived::domain::contract::types::*;
/// use ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase;
/// struct Item;
/// impl DerivedUseCase for Item {
///     fn build(&self, _prepared: &PreparedDerived, _inputs: DerivedInputs<'_>, _room: &mut BuildWorkspace, _recycled: Option<DerivedPayload>) -> Result<BuildOutcome, DerivedFault> { unimplemented!() }
///     fn view<'a>(&self, _prepared: &PreparedDerived, _payload: &'a DerivedPayload) -> Result<DerivedView<'a>, DerivedFault> { unimplemented!() }
/// }
/// ```
///
/// A missing `build` is refused.
///
/// ```compile_fail,E0046
/// use ryo_derived::domain::contract::types::*;
/// use ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase;
/// struct Item;
/// impl DerivedUseCase for Item {
///     fn prepare(&self, _request: ViewRequest, _schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>) -> std::sync::Arc<PreparedDerived> { unimplemented!() }
///     fn view<'a>(&self, _prepared: &PreparedDerived, _payload: &'a DerivedPayload) -> Result<DerivedView<'a>, DerivedFault> { unimplemented!() }
/// }
/// ```
///
/// A missing `view` is refused.
///
/// ```compile_fail,E0046
/// use ryo_derived::domain::contract::types::*;
/// use ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase;
/// struct Item;
/// impl DerivedUseCase for Item {
///     fn prepare(&self, _request: ViewRequest, _schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>) -> std::sync::Arc<PreparedDerived> { unimplemented!() }
///     fn build(&self, _prepared: &PreparedDerived, _inputs: DerivedInputs<'_>, _room: &mut BuildWorkspace, _recycled: Option<DerivedPayload>) -> Result<BuildOutcome, DerivedFault> { unimplemented!() }
/// }
/// ```
///
/// A view that takes inputs is refused: it cannot acquire or build.
///
/// ```compile_fail,E0050
/// use ryo_derived::domain::contract::types::*;
/// use ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase;
/// struct Item;
/// impl DerivedUseCase for Item {
///     fn prepare(&self, _request: ViewRequest, _schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>) -> std::sync::Arc<PreparedDerived> { unimplemented!() }
///     fn build(&self, _prepared: &PreparedDerived, _inputs: DerivedInputs<'_>, _room: &mut BuildWorkspace, _recycled: Option<DerivedPayload>) -> Result<BuildOutcome, DerivedFault> { unimplemented!() }
///     fn view<'a>(&self, _prepared: &PreparedDerived, _payload: &'a DerivedPayload, _inputs: DerivedInputs<'_>) -> Result<DerivedView<'a>, DerivedFault> { unimplemented!() }
/// }
/// ```
///
/// A view cannot outlive the payload it borrows.
///
/// ```compile_fail,E0515
/// use ryo_derived::domain::contract::types::*;
/// use ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase;
/// fn escape<'a>(item: &dyn DerivedUseCase, prepared: &PreparedDerived, payload: DerivedPayload) -> DerivedView<'a> {
///     item.view(prepared, &payload).unwrap()
/// }
/// ```
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
