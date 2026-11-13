//! The `derived` port, implemented by the derived crate's `prepare`, `build` and `view` use cases: requests, payloads and faults are
//! the values the core names, so the adapter passes them through. The calls are written in full because the use case traits and the
//! port share their method names.
use super::types::DerivedUseCases;
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::contract::types::{
    BuildInputs, BuildOutcome, BuildWorkspace, DerivedView,
};
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::inbound::use_cases::build::interface::BuildUseCase;
use ryo_derived::inbound::use_cases::prepare_derived::interface::PrepareDerivedUseCase;
use ryo_derived::inbound::use_cases::view::interface::ViewUseCase;
use ryo_relations::domain::relations::types::CellKind;

/// The `use_case` `derived` adapter.
#[derive(Debug)]
pub(crate) struct DerivedUseCaseAdapter {
    /// The three derived use cases, which the edge built.
    use_cases: DerivedUseCases,
}

impl DerivedUseCaseAdapter {
    /// The adapter over the three derived use cases the edge built.
    pub(crate) const fn new(use_cases: DerivedUseCases) -> Self {
        Self { use_cases }
    }
}

impl DerivedPort for DerivedUseCaseAdapter {
    fn prepare(&self, request: DerivedRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived {
        PrepareDerivedUseCase::prepare_derived(
            &*self.use_cases.prepare,
            request,
            ryo_derived::domain::contract::types::InputSchemas(schemas),
        )
    }
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: BuildInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault> {
        let recycled = match recycled {
            Some(payload) => ryo_derived::domain::contract::types::Recycled::Payload(payload),
            None => ryo_derived::domain::contract::types::Recycled::Nothing,
        };
        BuildUseCase::build(&*self.use_cases.build, prepared, inputs, room, recycled)
    }
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        ViewUseCase::view(&*self.use_cases.view, prepared, payload)
    }
}
