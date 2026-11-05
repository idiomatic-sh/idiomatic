//! JSON conversions delegate validation to adjacent schemas.
use super::schemas;
use crate::outbound::collection::{interface::CollectionPort, types::CollectorMessage};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
#[derive(Debug)]
/// Validates collector wire messages independently of process execution.
pub(crate) struct CollectionJsonAdapter;
impl CollectionPort for CollectionJsonAdapter {
    fn request(&self, invocation: &Invocation) -> Result<CollectorMessage, Error> {
        schemas::request(invocation)
    }
    fn invocation(&self, message: &CollectorMessage) -> Result<Invocation, Error> {
        schemas::invocation(message)
    }
    fn response(
        &self,
        observation: Result<ProcessOutput, Error>,
    ) -> Result<CollectorMessage, Error> {
        schemas::response(observation)
    }
    fn observation(
        &self,
        message: &CollectorMessage,
    ) -> Result<Result<ProcessOutput, Error>, Error> {
        schemas::observation(message)
    }
}
