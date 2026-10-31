//! JSON exists only at the child-process boundary.
use crate::domain::diagnostics::{
    errors::DiagnosticsError as Error,
    types::{BenchmarkFinding, RunReport},
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

impl From<WorkerRequest> for RyoWorkerRequestSchema {
    fn from(request: WorkerRequest) -> Self {
        Self {
            root: request.root,
            rules: request.rules,
            files: request
                .files
                .into_iter()
                .map(|path| path.0.into_string())
                .collect(),
            cache: match request.state {
                crate::domain::suite::types::WorkerState::Fresh => None,
                crate::domain::suite::types::WorkerState::Retained(path) => Some(path),
            },
        }
    }
}
impl TryFrom<RyoReportSchema> for RunReport {
    type Error = Error;
    fn try_from(wire: RyoReportSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            files: wire
                .files
                .into_iter()
                .map(|path| ryo::linter::RelativePath(path.into_boxed_str()))
                .collect(),
            diagnostics: wire
                .diagnostics
                .into_iter()
                .map(|finding| {
                    Ok(BenchmarkFinding {
                        rule: ryo::linter::RuleId::try_from(finding.rule.as_str())
                            .map_err(|error| Error::Invalid(error.to_string()))?,
                        file: ryo::linter::RelativePath(finding.file.into_boxed_str()),
                        start: crate::domain::diagnostics::types::SourceByteOffset(finding.start),
                        end: crate::domain::diagnostics::types::SourceByteOffset(finding.end),
                    })
                })
                .collect::<Result<_, Error>>()?,
        })
    }
}
pub(super) fn encode_request(request: WorkerRequest) -> Result<String, Error> {
    serde_json::to_string(&RyoWorkerRequestSchema::from(request))
        .map_err(|error| Error::Json(Box::new(error)))
}
pub(super) fn decode_report(bytes: &[u8]) -> Result<RunReport, Error> {
    let wire: RyoReportSchema =
        serde_json::from_slice(bytes).map_err(|error| Error::Json(Box::new(error)))?;
    wire.try_into()
}
#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
