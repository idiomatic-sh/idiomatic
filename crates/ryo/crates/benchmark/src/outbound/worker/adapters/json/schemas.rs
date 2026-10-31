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
            files: request
                .files
                .into_iter()
                .map(|path| ryo::linter::RelativePath(path.into_boxed_str()))
                .collect(),
            state: request.cache.map_or(
                crate::domain::suite::types::WorkerState::Fresh,
                crate::domain::suite::types::WorkerState::Retained,
            ),
        }
    }
}
#[derive(Debug, Serialize)]
struct ReportSchema<'a> {
    diagnostics: Vec<DiagnosticSchema<'a>>,
    files: Vec<&'a str>,
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
        files: report.files.iter().map(|path| path.0.as_ref()).collect(),
        diagnostics: report
            .diagnostics
            .iter()
            .map(|finding| DiagnosticSchema {
                rule: finding.rule.as_ref(),
                file: finding.file.0.as_ref(),
                start: finding.start.0,
                end: finding.end.0,
            })
            .collect(),
    };
    Ok(WorkerResponseBytes(serde_json::to_vec(&wire)?))
}
#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
