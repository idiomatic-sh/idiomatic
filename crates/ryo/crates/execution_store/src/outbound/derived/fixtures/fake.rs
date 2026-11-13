//! Controlled stand-in for the derived port; records the calls it answers.
use crate::outbound::derived::factory::semantics;
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::contract::types::{
    BuildInputs, BuildOutcome, BuildWorkspace, DerivedView,
};
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The derived port answered by the portable implementations, counting each call.
#[derive(Debug, Default)]
pub struct DerivedFake {
    /// Number of derived calls made by the engine.
    pub calls: AtomicUsize,
}

impl DerivedPort for DerivedFake {
    fn prepare(&self, request: DerivedRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived {
        self.calls.fetch_add(1, Ordering::Relaxed);
        semantics().prepare(request, schemas)
    }
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: BuildInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        semantics().build(prepared, inputs, room, recycled)
    }
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        semantics().view(prepared, payload)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
