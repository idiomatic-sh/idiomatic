//! Typed in-memory protocol double, independent of JSON serialization.
use super::super::{interface::CollectionPort, types::CollectorMessage};
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::{Invocation, ProcessOutput},
};
use std::sync::Mutex;
#[derive(Clone, Debug)]
/// Cloneable failure representation retained by the in-memory protocol.
enum Failure {
    /// Original operating-system launch error number.
    Os(i32),
    /// IO failure without a raw operating-system number.
    Kind(std::io::ErrorKind, String),
    /// Non-IO collection or validation failure.
    Other(String),
}
#[derive(Debug, Default)]
/// Deterministic protocol storage used by orchestration and shared contract tests.
pub struct CollectionFake {
    /// Last successfully encoded request.
    request: Mutex<Option<Invocation>>,
    /// Last successfully encoded observation or failure.
    response: Mutex<Option<Result<ProcessOutput, Failure>>>,
}
impl CollectionPort for CollectionFake {
    #[expect(
        clippy::expect_used,
        reason = "a poisoned protocol fixture lock must fail the test"
    )]
    fn request(&self, invocation: &Invocation) -> Result<CollectorMessage, Error> {
        *self.request.lock().expect("request lock") = Some(invocation.clone());
        Ok(CollectorMessage(b"request".to_vec()))
    }
    #[expect(
        clippy::expect_used,
        reason = "a poisoned protocol fixture lock must fail the test"
    )]
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
    #[expect(
        clippy::expect_used,
        reason = "a poisoned protocol fixture lock must fail the test"
    )]
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
                error @ (Error::Invalid(_) | Error::Json(_) | Error::Toml(_)) => {
                    Failure::Other(error.to_string())
                }
            }));
        Ok(CollectorMessage(b"response".to_vec()))
    }
    #[expect(
        clippy::expect_used,
        reason = "a poisoned protocol fixture lock must fail the test"
    )]
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
            Failure::Other(detail) => Error::Invalid(detail),
            Failure::Kind(kind, detail) => std::io::Error::new(kind, detail).into(),
        }))
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
