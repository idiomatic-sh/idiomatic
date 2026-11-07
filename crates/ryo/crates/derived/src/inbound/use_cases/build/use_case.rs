//! The `build` use case: stateless; concrete construction belongs at a composition edge.
use super::interface::BuildUseCase;
use crate::domain::contract::types::derived_payload::value::DerivedPayload;
use crate::domain::contract::types::prepared_derived::value::PreparedDerived;
use crate::domain::contract::types::{BuildInputs, BuildOutcome, BuildWorkspace};
use crate::domain::fault::types::DerivedFault;

/// `build`, over no port: it holds no state.
#[derive(Clone, Copy, Debug)]
pub struct BuildUseCaseImpl;

impl BuildUseCaseImpl {
    /// `build`, which holds nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl BuildUseCase for BuildUseCaseImpl {
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: BuildInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault> {
        crate::domain::dispatch::service::build(prepared, inputs, room, recycled)
    }
}
