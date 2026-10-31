//! A low-footprint collector launches and observes the actual contestant.
use super::{constants, schemas, settings::ExecutionTimedSettings};
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::{Invocation, ProcessOutput},
};
use crate::outbound::execution::interface::ExecutionPort;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[derive(Debug)]
pub(crate) struct ExecutionTimedAdapter {
    settings: ExecutionTimedSettings,
}
impl ExecutionTimedAdapter {
    pub(crate) fn new(settings: ExecutionTimedSettings) -> Self {
        Self { settings }
    }
}
impl ExecutionPort for ExecutionTimedAdapter {
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let bytes = std::fs::read(&self.settings.executable)?;
        if format!("{:x}", Sha256::digest(&bytes)) != self.settings.digest.0 {
            return Err(Error::Invalid(constants::DIGEST.into()));
        }
        drop(bytes);
        let request = schemas::request(&invocation)?;
        let mut child = Command::new(&self.settings.executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut input = child
            .stdin
            .take()
            .ok_or_else(|| Error::Invalid(constants::PIPE.into()))?;
        let output = std::thread::scope(|scope| -> Result<_, Error> {
            let writer = scope.spawn(move || input.write_all(&request.0));
            let output = child.wait_with_output()?;
            writer
                .join()
                .map_err(|_| Error::Invalid(constants::WRITER.into()))??;
            Ok(output)
        })?;
        if !output.status.success() {
            return Err(Error::Invalid(format!(
                "{}: {}",
                constants::FAILED,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        schemas::observation(&crate::domain::measurements::types::CapturedBytes(
            output.stdout,
        ))?
    }
}
