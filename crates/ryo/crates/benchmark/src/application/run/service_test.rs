//! Coverage failures must invalidate results even when both tools report no findings.
use super::service::run;
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::domain::{
    diagnostics::{errors::Error, types::RunReport},
    suite::types::RunRequest,
};
use crate::outbound::{execution::interface::ExecutionPort, linter::interface::LinterPort};
struct Empty;
impl LinterPort for Empty {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        Ok(Invocation {
            executable: "unused".into(),
            arguments: vec![],
            environment: Default::default(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, _: &RunRequest, _: ProcessOutput) -> Result<RunReport, Error> {
        Ok(RunReport {
            diagnostics: vec![],
            files: vec![],
        })
    }
}
impl ExecutionPort for Empty {
    fn run(&self, _: Invocation) -> Result<ProcessOutput, Error> {
        Ok(ProcessOutput {
            status: 0,
            stdout: vec![],
            stderr: vec![],
            wall: std::time::Duration::ZERO,
            resources: None,
        })
    }
}
#[test]
fn omitted_input_is_an_error() {
    let request = RunRequest {
        root: ".".into(),
        cache: "cache".into(),
        files: vec!["a.py".into()],
    };
    assert!(run(&Empty, &Empty, &request).is_err());
}
