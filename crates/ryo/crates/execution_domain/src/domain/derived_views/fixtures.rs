//! The portable derived use cases the domain's tests call.
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::inbound::use_cases::prepare_derived::interface::PrepareDerivedUseCase;
use ryo_derived::inbound::use_cases::prepare_derived::use_case::PrepareDerivedUseCaseImpl;

/// The portable derived use cases the domain's tests call, as one value: the tests never reach the engine's outbound port.
pub(crate) struct Semantics {
    prepare: PrepareDerivedUseCaseImpl,
}

/// The portable derived use cases, built where the test needs them.
pub(crate) fn semantics() -> Semantics {
    Semantics {
        prepare: PrepareDerivedUseCaseImpl::new(),
    }
}

impl Semantics {
    /// Checks a request against the schemas of its inputs.
    pub(crate) fn prepare(
        &self,
        request: DerivedRequest,
        schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    ) -> PreparedDerived {
        PrepareDerivedUseCase::prepare_derived(
            &self.prepare,
            request,
            ryo_derived::domain::contract::types::InputSchemas(schemas),
        )
    }
}
