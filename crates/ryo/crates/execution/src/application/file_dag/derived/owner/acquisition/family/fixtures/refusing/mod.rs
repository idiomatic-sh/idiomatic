//! A real portable use case whose build is the one injected dependency failure.
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::contract::types::BuildInputs as DerivedInputs;
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::request::types::DerivedRequest as ViewRequest;
use crate::outbound::derived::interface::DerivedPort;
use ryo_relations::domain::relations::types::CellKind;
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct RefusingBuild;
impl DerivedPort for RefusingBuild {
    fn prepare(&self, request: ViewRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived {
        crate::outbound::derived::fixtures::fake::DerivedFake::default().prepare(request, schemas)
    }
    fn build(
        &self,
        _: &PreparedDerived,
        _: DerivedInputs<'_>,
        _: &mut BuildWorkspace,
        _: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault> {
        Err(DerivedFault::DuplicateOrder)
    }
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        crate::outbound::derived::fixtures::fake::DerivedFake::default().view(prepared, payload)
    }
}

