//! Application composition and outer file/project execution. No provider or
//! dependency resolver is exposed through the semantic kernel port.
use super::super::executor::PrimitiveExecutor;
use crate::outbound::derived::interface::DerivedPort;
use std::sync::Arc;

pub(crate) struct ProgramServices<'service> {
    kernels: &'service dyn PrimitiveExecutor,
    derived: Arc<dyn DerivedPort>,
}
impl<'service> ProgramServices<'service> {
    pub(crate) fn new(
        kernels: &'service dyn PrimitiveExecutor,
        derived: Arc<dyn DerivedPort>,
    ) -> Self {
        Self { kernels, derived }
    }
    pub(super) fn kernels(&self) -> &'service dyn PrimitiveExecutor {
        self.kernels
    }
    pub(super) fn derived(&self) -> &Arc<dyn DerivedPort> {
        &self.derived
    }
}
