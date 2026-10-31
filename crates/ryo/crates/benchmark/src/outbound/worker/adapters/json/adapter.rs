//! The worker port, implemented with an immutable request and injected output connection.
use super::{connection::WorkerJsonConnection, schemas};
use crate::domain::diagnostics::{errors::DiagnosticsError as Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::worker::interface::WorkerPort;
use std::{ffi::OsString, io::Write};

#[derive(Debug)]
/// Receives one explicit request and publishes reports to the injected stream.
pub(crate) struct WorkerJsonAdapter {
    /// Immutable command-line request payload.
    payload: OsString,
    /// Synchronized output connection owned by this adapter.
    connection: WorkerJsonConnection,
}
impl WorkerJsonAdapter {
    /// Bind the request and output stream supplied by executable composition.
    pub(crate) fn new(payload: OsString, output: Box<dyn Write + Send>) -> Self {
        Self {
            payload,
            connection: WorkerJsonConnection::new(output),
        }
    }
}
impl WorkerPort for WorkerJsonAdapter {
    fn receive(&self) -> Result<WorkerRequest, Error> {
        schemas::request(self.payload.as_os_str())
            .map_err(|error| Error::Invalid(error.to_string()))
    }
    fn respond(&self, report: &RunReport) -> Result<(), Error> {
        let bytes = schemas::report(report).map_err(|error| Error::Invalid(error.to_string()))?;
        self.connection.publish(&bytes)
    }
}
