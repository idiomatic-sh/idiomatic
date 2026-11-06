//! A real portable use case whose build is the one injected dependency failure.
use ryo_derived::domain::contract::types::{
    BuildOutcome, BuildWorkspace, DerivedFault, DerivedInputs, DerivedPayload, DerivedView,
    PreparedDerived, ViewRequest,
};
use ryo_derived::inbound::use_cases::derived::{
    interface::DerivedUseCase, use_case::DerivedUseCaseImpl,
};
use ryo_relations::domain::relations::types::CellKind;
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct RefusingBuild;
impl DerivedUseCase for RefusingBuild {
    fn prepare(&self, request: ViewRequest, schemas: Vec<Vec<CellKind>>) -> Arc<PreparedDerived> {
        DerivedUseCaseImpl::new().prepare(request, schemas)
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
        DerivedUseCaseImpl::new().view(prepared, payload)
    }
}
