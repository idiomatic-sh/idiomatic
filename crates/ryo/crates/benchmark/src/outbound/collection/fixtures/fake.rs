//! Typed in-memory protocol double, independent of JSON serialization.
use super::super::{interface::CollectionPort, types::CollectorMessage};
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::{Invocation, ProcessOutput},
};
use std::sync::Mutex;
#[derive(Clone, Debug)]
enum Failure {
    Os(i32),
    Kind(std::io::ErrorKind, String),
    Other(String),
}
#[derive(Debug, Default)]
/// Deterministic protocol storage used by orchestration and shared contract tests.
pub struct CollectionFake {
    request: Mutex<Option<Invocation>>,
    response: Mutex<Option<Result<ProcessOutput, Failure>>>,
}
impl CollectionPort for CollectionFake {
    fn request(&self, invocation: &Invocation) -> Result<CollectorMessage, Error> {
        *self.request.lock().expect("request lock") = Some(invocation.clone());
        Ok(CollectorMessage(b"request".to_vec()))
    }
    fn invocation(&self, message: &CollectorMessage) -> Result<Invocation, Error> {
        if message.0 != b"request" {
            return Err(Error::Invalid("malformed request".into()));
        }
        self.request
            .lock()
            .expect("request lock")
            .clone()
            .ok_or_else(|| Error::Invalid("missing request".into()))
    }
    fn response(
        &self,
        observation: Result<ProcessOutput, Error>,
    ) -> Result<CollectorMessage, Error> {
        *self.response.lock().expect("response lock") =
            Some(observation.map_err(|error| match error {
                Error::Io(error) => match error.raw_os_error() {
                    Some(errno) => Failure::Os(errno),
                    None => Failure::Kind(error.kind(), error.to_string()),
                },
                error => Failure::Other(error.to_string()),
            }));
        Ok(CollectorMessage(b"response".to_vec()))
    }
    fn observation(
        &self,
        message: &CollectorMessage,
    ) -> Result<Result<ProcessOutput, Error>, Error> {
        if message.0 != b"response" {
            return Err(Error::Invalid("malformed response".into()));
        }
        let result = self
            .response
            .lock()
            .expect("response lock")
            .clone()
            .ok_or_else(|| Error::Invalid("missing response".into()))?;
        Ok(result.map_err(|error| match error {
            Failure::Os(errno) => std::io::Error::from_raw_os_error(errno).into(),
            Failure::Other(message) => Error::Invalid(message),
            Failure::Kind(kind, message) => std::io::Error::new(kind, message).into(),
        }))
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
