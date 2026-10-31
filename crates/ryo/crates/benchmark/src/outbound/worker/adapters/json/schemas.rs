//! Private wire shapes; neither canonical values nor callers depend on serde.
use super::types::WorkerResponseBytes;
use crate::domain::diagnostics::types::RunReport;
use crate::domain::suite::types::WorkerRequest;
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub(super) enum WorkerSchemaError {
    #[error("worker request is not UTF-8")]
    NotText,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestSchema {
    root: PathBuf,
    rules: PathBuf,
    files: Vec<String>,
    cache: Option<PathBuf>,
}
impl From<RequestSchema> for WorkerRequest {
    fn from(request: RequestSchema) -> Self {
        Self {
            root: request.root,
            rules: request.rules,
            files: request.files,
            cache: request.cache,
        }
    }
}
#[derive(Debug, Serialize)]
struct ReportSchema<'a> {
    diagnostics: Vec<DiagnosticSchema<'a>>,
    files: &'a [String],
}
#[derive(Debug, Serialize)]
struct DiagnosticSchema<'a> {
    rule: &'a str,
    file: &'a str,
    start: usize,
    end: usize,
}
pub(super) fn request(payload: &OsStr) -> Result<WorkerRequest, WorkerSchemaError> {
    let text = payload.to_str().ok_or(WorkerSchemaError::NotText)?;
    let request: RequestSchema = serde_json::from_str(text)?;
    Ok(request.into())
}
pub(super) fn report(report: &RunReport) -> Result<WorkerResponseBytes, WorkerSchemaError> {
    let wire = ReportSchema {
        files: &report.files,
        diagnostics: report
            .diagnostics
            .iter()
            .map(|finding| DiagnosticSchema {
                rule: &finding.rule,
                file: &finding.file,
                start: finding.start,
                end: finding.end,
            })
            .collect(),
    };
    Ok(WorkerResponseBytes(serde_json::to_vec(&wire)?))
}
#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
