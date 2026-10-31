//! JSON conversions delegate validation to adjacent schemas.
use super::schemas;
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::{Invocation, ProcessOutput},
};
use crate::outbound::collection::{interface::CollectionPort, types::CollectorMessage};
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
