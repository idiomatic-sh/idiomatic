//! The worker port, implemented with an immutable request and injected output connection.
use super::{
    connection::WorkerJsonConnection,
    types::{WorkerPayload, WorkerResponseBytes},
};
use crate::outbound::worker::{interface::WorkerPort, types::WorkerSource};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
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
impl WorkerPort for WorkerJsonAdapter {
    fn receive(&self) -> Result<WorkerRequest, Error> {
        let payload = match &self.source {
            WorkerSource::Argument(payload) => payload.clone(),
            WorkerSource::Stored(path) => OsString::from(std::fs::read_to_string(path)?),
        };
        WorkerRequest::try_from(WorkerPayload(payload.as_os_str()))
            .map_err(|error| Error::Invalid(error.to_string()))
    }
    fn respond(&self, report: &RunReport) -> Result<(), Error> {
        let bytes = WorkerResponseBytes::try_from(report)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        self.connection.publish(&bytes)
    }
}
