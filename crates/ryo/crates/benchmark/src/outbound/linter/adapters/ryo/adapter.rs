//! Serialized requests avoid shell interpretation and preserve the input list.
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::domain::suite::types::WorkerRequest;
use crate::domain::{
    diagnostics::{errors::DiagnosticsError as Error, types::RunReport},
    suite::types::RunRequest,
};
use crate::outbound::linter::interface::LinterPort;
use std::path::PathBuf;
#[derive(Debug)]
/// Constructs explicit candidate invocations and validates their reports.
pub(crate) struct LinterRyoAdapter {
    /// Pinned candidate executable.
    pub executable: PathBuf,
    /// Frozen directory of selected rule queries.
    pub rules: PathBuf,
    /// Explicit logical query-module registry under the frozen profile root.
    pub query_modules: std::collections::BTreeMap<String, PathBuf>,
    /// Parser configuration forwarded to the isolated SDK worker.
    pub parser_configuration: Option<String>,
}
impl LinterPort for LinterRyoAdapter {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        let worker = WorkerRequest {
            root: request.root.clone(),
            rules: self.rules.clone(),
            query_modules: self.query_modules.clone(),
            parser_configuration: self.parser_configuration.clone(),
            files: request.files.clone(),
            state: crate::domain::suite::types::WorkerState::Retained(
                request.state.join(super::constants::CACHE),
            ),
        };
        let mut arguments = vec![
            super::constants::WORKER.into(),
            super::schemas::encode_request(worker)?.into(),
        ];
        if let Some(configuration) = &self.parser_configuration {
            arguments.push(configuration.clone().into());
        }
        Ok(Invocation {
            executable: self.executable.clone(),
            arguments,
            environment: std::collections::BTreeMap::default(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, _: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        if output.status.0 != 0_i32 || !output.stderr.0.is_empty() {
            return Err(Error::Invalid(format!(
                "candidate failed ({}): {}",
                output.status.0,
                String::from_utf8_lossy(&output.stderr.0)
            )));
        }
        super::schemas::decode_report(&output.stdout.0)
    }
}
