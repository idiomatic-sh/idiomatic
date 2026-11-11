//! The owner-issued capability over the derived port.
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::request::types::DerivedRequest;
use std::sync::Arc;

/// The owner-issued capability over the derived port. Holders outside the owner can ask the
/// semantics to prepare a contract; only the owner can reach the port, so no holder can build,
/// and every view is built through admission, the ledger and a lease.
#[derive(Clone, Debug)]
pub struct DerivedAccess {
    port: Arc<dyn DerivedPort>,
}

impl DerivedAccess {
    /// Request-time metadata only: preparing never builds or retains a view.
    pub fn prepare(
        &self,
        request: DerivedRequest,
        schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    ) -> Arc<PreparedDerived> {
        Arc::new(self.port.prepare(request, schemas))
    }
    /// The builder is reachable only inside the owner tree.
    pub(crate) fn port(&self) -> &Arc<dyn DerivedPort> {
        &self.port
    }
}

impl From<Arc<dyn DerivedPort>> for DerivedAccess {
    fn from(port: Arc<dyn DerivedPort>) -> Self {
        Self { port }
    }
}

impl<T: DerivedPort + 'static> From<Arc<T>> for DerivedAccess {
    fn from(port: Arc<T>) -> Self {
        Self { port }
    }
}
