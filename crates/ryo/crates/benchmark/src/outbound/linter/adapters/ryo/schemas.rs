//! JSON exists only at the child-process boundary.
use crate::domain::diagnostics::{
    errors::Error,
    types::{Diagnostic, RunReport},
};
use crate::domain::suite::types::WorkerRequest;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RyoWorkerRequestSchema {
    root: PathBuf,
    rules: PathBuf,
    files: Vec<String>,
    cache: Option<PathBuf>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RyoReportSchema {
    diagnostics: Vec<RyoDiagnosticSchema>,
    files: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RyoDiagnosticSchema {
    rule: String,
    file: String,
    start: usize,
    end: usize,
}

pub(super) fn encode_request(request: WorkerRequest) -> Result<String, Error> {
    Ok(serde_json::to_string(&RyoWorkerRequestSchema {
        root: request.root,
        rules: request.rules,
        files: request.files,
        cache: request.cache,
    })?)
}
pub(super) fn decode_report(bytes: &[u8]) -> Result<RunReport, Error> {
    let wire: RyoReportSchema = serde_json::from_slice(bytes)?;
    Ok(RunReport {
        files: wire.files,
        diagnostics: wire
            .diagnostics
            .into_iter()
            .map(|finding| Diagnostic {
                rule: finding.rule,
                file: finding.file,
                start: finding.start,
                end: finding.end,
            })
            .collect(),
    })
}
