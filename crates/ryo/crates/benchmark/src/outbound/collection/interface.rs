//! Collector protocol preserves native arguments and actual launch errors.
use super::types::CollectorMessage;
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::{Invocation, ProcessOutput},
};
/// Encode and validate the protocol independently of execution.
pub trait CollectionPort {
    /// Encode a native invocation. Errors if serialization fails.
    fn request(&self, invocation: &Invocation) -> Result<CollectorMessage, Error>;
    /// Validate and decode an invocation. Errors for malformed messages.
    fn invocation(&self, message: &CollectorMessage) -> Result<Invocation, Error>;
    /// Encode the actual child result, including launch failure. Errors if serialization fails.
    fn response(
        &self,
        observation: Result<ProcessOutput, Error>,
    ) -> Result<CollectorMessage, Error>;
    /// Validate a collector result. Outer errors mean malformed protocol; inner errors mean child launch or observation failure.
    fn observation(
        &self,
        message: &CollectorMessage,
    ) -> Result<Result<ProcessOutput, Error>, Error>;
}
