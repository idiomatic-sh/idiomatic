//! The portable derived use cases the domain's tests call.
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::contract::types::{
    BuildInputs, BuildOutcome, BuildWorkspace, DerivedView,
};
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::inbound::use_cases::build::interface::BuildUseCase;
use ryo_derived::inbound::use_cases::build::use_case::BuildUseCaseImpl;
use ryo_derived::inbound::use_cases::prepare_derived::interface::PrepareDerivedUseCase;
use ryo_derived::inbound::use_cases::prepare_derived::use_case::PrepareDerivedUseCaseImpl;
use ryo_derived::inbound::use_cases::view::interface::ViewUseCase;
use ryo_derived::inbound::use_cases::view::use_case::ViewUseCaseImpl;

/// The portable derived use cases the domain's tests call, as one value: the tests never reach the engine's outbound port.
pub(crate) struct Semantics {
    prepare: PrepareDerivedUseCaseImpl,
    build: BuildUseCaseImpl,
    view: ViewUseCaseImpl,
}

/// The portable derived use cases, built where the test needs them.
pub(crate) fn semantics() -> Semantics {
    Semantics {
        prepare: PrepareDerivedUseCaseImpl::new(),
        build: BuildUseCaseImpl::new(),
        view: ViewUseCaseImpl::new(),
    }
}

impl Semantics {
    /// Checks a request against the schemas of its inputs.
    pub(crate) fn prepare(
        &self,
        request: DerivedRequest,
        schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    ) -> PreparedDerived {
        PrepareDerivedUseCase::prepare_derived(&self.prepare, request, schemas)
    }

    /// Builds the payload of a prepared request.
    pub(crate) fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: BuildInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault> {
        BuildUseCase::build(&self.build, prepared, inputs, room, recycled)
    }

    /// A view over a built payload.
    pub(crate) fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        ViewUseCase::view(&self.view, prepared, payload)
    }
}
