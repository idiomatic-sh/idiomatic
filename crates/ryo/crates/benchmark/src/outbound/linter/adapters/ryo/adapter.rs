//! Serialized requests avoid shell interpretation and preserve the input list.
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::domain::suite::types::WorkerRequest;
use crate::domain::{
    diagnostics::{errors::Error, types::RunReport},
    suite::types::RunRequest,
};
use crate::outbound::linter::interface::LinterPort;
use std::path::PathBuf;
#[derive(Debug)]
pub struct LinterRyoAdapter {
    pub executable: PathBuf,
    pub rules: PathBuf,
}
impl LinterPort for LinterRyoAdapter {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        let worker = WorkerRequest {
            root: request.root.clone(),
            rules: self.rules.clone(),
            files: request.files.clone(),
            cache: Some(request.cache.join("ryo")),
        };
        Ok(Invocation {
            executable: self.executable.clone(),
            arguments: vec![
                "run-ryo".into(),
                super::schemas::encode_request(worker)?.into(),
            ],
            environment: Default::default(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, _: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        if output.status != 0 || !output.stderr.is_empty() {
            return Err(Error::Invalid(format!(
                "candidate failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        super::schemas::decode_report(&output.stdout)
    }
}
