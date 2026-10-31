//! Linter integrations bind commands to decoding; execution stays shared.
use crate::domain::diagnostics::{errors::Error, types::RunReport};
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::domain::suite::types::RunRequest;

pub trait LinterPort {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error>;
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error>;
}
