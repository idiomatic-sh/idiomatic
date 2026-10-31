//! Single child observation operation.
use crate::domain::diagnostics::errors::DiagnosticsError;
use crate::outbound::collection::types::CollectorMessage;
/// Decode, launch, observe and encode one actual child.
pub trait CollectUseCase {
    /// Execute the collector protocol. Errors denote malformed requests or response failures.
    fn collect(&self, request: &CollectorMessage) -> Result<CollectorMessage, DiagnosticsError>;
}
