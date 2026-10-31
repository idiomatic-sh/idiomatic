//! GNU time owns wait/rusage collection; streams remain contestant-owned.
use super::{constants, schemas, settings::ExecutionTimedSettings};
use crate::domain::{
    diagnostics::errors::Error,
    measurements::types::{Invocation, ProcessOutput},
};
use crate::outbound::execution::interface::ExecutionPort;
use sha2::{Digest, Sha256};
use std::{process::Command, time::Instant};

#[derive(Debug)]
pub(crate) struct ExecutionTimedAdapter {
    settings: ExecutionTimedSettings,
}

impl ExecutionTimedAdapter {
    /// Bind the explicitly pinned instrumentation executable.
    pub(crate) fn new(settings: ExecutionTimedSettings) -> Self {
        Self { settings }
    }
}

impl ExecutionPort for ExecutionTimedAdapter {
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let digest = format!(
            "{:x}",
            Sha256::digest(std::fs::read(&self.settings.executable)?)
        );
        if digest != self.settings.sha256.0 {
            return Err(Error::Invalid(constants::PIN_MISMATCH.into()));
        }
        // Resolve a named child before wrapping it so launch failures retain the port contract.
        let executable = if invocation.executable.components().count() > 1
            || invocation.executable.is_absolute()
        {
            let path = if invocation.executable.is_absolute() {
                invocation.executable.clone()
            } else {
                invocation.directory.join(&invocation.executable)
            };
            let _metadata = std::fs::metadata(&path)?;
            path
        } else {
            let search = invocation
                .environment
                .get(std::ffi::OsStr::new(constants::PATH))
                .cloned()
                .or_else(|| self.settings.path.clone())
                .unwrap_or_default();
            std::env::split_paths(&search)
                .map(|entry| {
                    let base = if entry.is_absolute() {
                        entry
                    } else {
                        invocation.directory.join(entry)
                    };
                    base.join(&invocation.executable)
                })
                .find(|path| path.is_file())
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, constants::MISSING_CHILD)
                })?
        };
        let metrics = tempfile::NamedTempFile::new()?;
        let mut command = Command::new(&self.settings.executable);
        let _ = command
            .args([
                constants::RESOURCE_QUIET,
                constants::FORMAT_FLAG,
                constants::FORMAT,
                constants::OUTPUT,
            ])
            .arg(metrics.path())
            .arg(constants::SEPARATOR)
            .arg(executable)
            .args(invocation.arguments)
            .envs(invocation.environment)
            .env(constants::LOCALE, constants::PORTABLE_LOCALE)
            .current_dir(invocation.directory);
        let start = Instant::now();
        let output = command.output()?;
        let wall = start.elapsed();
        let status = output
            .status
            .code()
            .ok_or_else(|| Error::Invalid(constants::SIGNAL.into()))?;
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
