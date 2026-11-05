//! Single child observation operation.
use crate::outbound::collection::types::CollectorMessage;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
/// Decode, launch, observe and encode one actual child.
pub trait CollectUseCase {
    /// Execute the collector protocol. Errors denote malformed requests or response failures.
    ///
    /// # Errors
    /// Refuses malformed protocol data and propagates response encoding or execution failures.
    fn collect(&self, request: &CollectorMessage) -> Result<CollectorMessage, ReportsError>;
}
