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
struct WorkerWire {
    root: PathBuf,
    rules: PathBuf,
    files: Vec<String>,
    cache: Option<PathBuf>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportWire {
    diagnostics: Vec<DiagnosticWire>,
    files: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticWire {
    rule: String,
    file: String,
    start: usize,
    end: usize,
}

pub fn encode_request(request: WorkerRequest) -> Result<String, Error> {
    Ok(serde_json::to_string(&WorkerWire {
        root: request.root,
        rules: request.rules,
        files: request.files,
        cache: request.cache,
    })?)
}
pub fn decode_request(text: &str) -> Result<WorkerRequest, Error> {
    let wire: WorkerWire = serde_json::from_str(text)?;
    Ok(WorkerRequest {
        root: wire.root,
        rules: wire.rules,
        files: wire.files,
        cache: wire.cache,
    })
}
pub fn encode_report(report: RunReport) -> Result<String, Error> {
    Ok(serde_json::to_string(&ReportWire {
        files: report.files,
        diagnostics: report
            .diagnostics
            .into_iter()
            .map(|finding| DiagnosticWire {
                rule: finding.rule,
                file: finding.file,
                start: finding.start,
                end: finding.end,
            })
            .collect(),
    })?)
}
pub fn decode_report(bytes: &[u8]) -> Result<RunReport, Error> {
    let wire: ReportWire = serde_json::from_slice(bytes)?;
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
