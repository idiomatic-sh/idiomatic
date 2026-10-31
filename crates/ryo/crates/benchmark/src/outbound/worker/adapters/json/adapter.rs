//! The worker port, implemented with an immutable request and injected output connection.
use super::{connection::WorkerJsonConnection, schemas};
use crate::domain::diagnostics::{errors::Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::worker::interface::WorkerPort;
use std::{ffi::OsString, io::Write};

#[derive(Debug)]
pub(crate) struct WorkerJsonAdapter {
    payload: OsString,
    connection: WorkerJsonConnection,
}
impl WorkerJsonAdapter {
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
