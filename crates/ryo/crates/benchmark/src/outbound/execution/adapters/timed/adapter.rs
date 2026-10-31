//! GNU time owns wait/rusage collection; streams remain contestant-owned.
use super::{schemas, settings::ExecutionTimedSettings};
use crate::domain::{
    diagnostics::errors::Error,
    measurements::types::{Invocation, ProcessOutput},
};
use crate::outbound::execution::interface::ExecutionPort;
use sha2::{Digest, Sha256};
use std::{process::Command, time::Instant};

#[derive(Debug)]
pub struct ExecutionTimedAdapter {
    pub(crate) settings: ExecutionTimedSettings,
}

impl ExecutionPort for ExecutionTimedAdapter {
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let digest = format!(
            "{:x}",
            Sha256::digest(std::fs::read(&self.settings.executable)?)
        );
        if digest != self.settings.sha256 {
            return Err(Error::Invalid(
                "instrumentation executable differs from pin".into(),
            ));
        }
        let metrics = tempfile::NamedTempFile::new()?;
        let mut command = Command::new(&self.settings.executable);
        command
            .args(["--quiet", "--format", schemas::FORMAT, "--output"])
            .arg(metrics.path())
            .arg("--")
            .arg(invocation.executable)
            .args(invocation.arguments)
            .envs(invocation.environment)
            .env("LC_ALL", "C")
            .current_dir(invocation.directory);
        let start = Instant::now();
        let output = command.output()?;
        let wall = start.elapsed();
        let status = output
            .status
            .code()
            .ok_or_else(|| Error::Invalid("instrumentation terminated by signal".into()))?;
        let resources = schemas::decode(&std::fs::read_to_string(metrics.path())?)?;
        Ok(ProcessOutput {
            status,
            stdout: output.stdout,
            stderr: output.stderr,
            wall,
            resources: Some(resources),
        })
    }
}
