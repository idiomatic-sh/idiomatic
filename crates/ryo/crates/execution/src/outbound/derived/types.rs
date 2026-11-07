//! The types of the `derived` port: plain data and its trait impls.
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::contract::types::{BuildInputs, BuildOutcome, BuildWorkspace, DerivedView};
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::inbound::use_cases::build::{interface::BuildUseCase, use_case::BuildUseCaseImpl};
use ryo_derived::inbound::use_cases::prepare::{
    interface::PrepareUseCase, use_case::PrepareUseCaseImpl,
};
use ryo_derived::inbound::use_cases::view::{interface::ViewUseCase, use_case::ViewUseCaseImpl};
use ryo_relations::domain::relations::types::CellKind;

/// The portable implementations of the three derived use cases, composed once at an edge.
#[derive(Clone, Copy, Debug)]
pub struct DerivedUseCases {
    /// The preparation of requests.
    pub prepare: PrepareUseCaseImpl,
    /// The build of payloads.
    pub build: BuildUseCaseImpl,
    /// The view of payloads.
    pub view: ViewUseCaseImpl,
}

impl PrepareUseCase for DerivedUseCases {
    fn prepare(&self, request: DerivedRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived {
        self.prepare.prepare(request, schemas)
    }
}

impl BuildUseCase for DerivedUseCases {
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: BuildInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault> {
        self.build.build(prepared, inputs, room, recycled)
    }
}

impl ViewUseCase for DerivedUseCases {
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        self.view.view(prepared, payload)
    }
}
