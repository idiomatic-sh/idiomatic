//! A real portable use case whose build is the one injected dependency failure.
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::BuildInputs;
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;

#[derive(Debug)]
pub(super) struct RefusingBuild;
impl DerivedPort for RefusingBuild {
    fn prepare(
        &self,
        request: DerivedRequest,
        schemas: ryo_derived::domain::contract::types::InputSchemas,
    ) -> PreparedDerived {
        crate::application::file_dag::fixtures::derived().prepare(request, schemas)
    }
    fn build(
        &self,
        _: &PreparedDerived,
        _: BuildInputs<'_>,
        _: &mut BuildWorkspace,
        _: ryo_derived::domain::contract::types::Recycled,
    ) -> Result<BuildOutcome, DerivedFault> {
        Err(DerivedFault::DuplicateOrder)
    }
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        crate::application::file_dag::fixtures::derived().view(prepared, payload)
    }
}
