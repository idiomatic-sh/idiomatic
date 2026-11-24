//! The worker port, implemented with an immutable request and injected output connection.
use super::{
    connection::WorkerJsonConnection,
    types::{WorkerPayload, WorkerResponseBytes},
};
use crate::outbound::worker::{interface::WorkerPort, types::WorkerSource};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::reports::types::RunReport;
use benchmark_contracts::domain::requests::types::WorkerRequest;
use std::{ffi::OsString, io::Write};

#[derive(Debug)]
/// Receives one explicit request and publishes reports to the injected stream.
pub(crate) struct WorkerJsonAdapter {
    /// Immutable request payload, inline or in a file.
    source: WorkerSource,
    /// Synchronized output connection owned by this adapter.
    connection: WorkerJsonConnection,
}
impl WorkerJsonAdapter {
    /// Bind the request and output stream supplied by executable composition.
    pub(crate) fn new(source: WorkerSource, output: Box<dyn Write + Send>) -> Self {
        Self {
            source,
            connection: WorkerJsonConnection::new(output),
        }
    }
}
impl WorkerJsonAdapter {
    /// The request `payload` holds.
    fn decoded(payload: &OsString) -> Result<WorkerRequest, Error> {
        WorkerRequest::try_from(WorkerPayload(payload.as_os_str()))
            .map_err(|error| Error::Invalid(error.to_string()))
    }
}
impl WorkerPort for WorkerJsonAdapter {
    fn receive(&self) -> Result<WorkerRequest, Error> {
        match &self.source {
            WorkerSource::Argument(payload) => Self::decoded(payload),
            WorkerSource::Stored(path) => self.read(path),
        }
    }
    fn read(&self, path: &std::path::Path) -> Result<WorkerRequest, Error> {
        Self::decoded(&OsString::from(std::fs::read_to_string(path)?))
    }
    fn encoded(
        &self,
        report: &RunReport,
    ) -> Result<crate::outbound::worker::types::EncodedResponse, Error> {
        let bytes = WorkerResponseBytes::try_from(report)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        Ok(super::connection::framed(&bytes))
    }
    fn respond(&self, report: &RunReport) -> Result<(), Error> {
        let bytes = WorkerResponseBytes::try_from(report)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        self.connection.publish(&bytes)
    }
}
