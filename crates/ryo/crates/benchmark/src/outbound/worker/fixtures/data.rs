//! Canonical request and response samples, kept apart from contract rows.
use crate::domain::{
    diagnostics::types::{Diagnostic, RunReport},
    suite::types::WorkerRequest,
};
/// The request fixture both concrete transports and doubles receive.
#[must_use]
pub fn request() -> WorkerRequest {
    WorkerRequest {
        root: "workspace".into(),
        rules: "rules".into(),
        files: vec!["source.txt".into()],
        cache: Some("cache".into()),
    }
}
/// A report with duplicated findings, which encoding must preserve.
#[must_use]
pub fn report() -> RunReport {
    let finding = Diagnostic {
        rule: "fixture".into(),
        file: "source.txt".into(),
        start: 2,
        end: 4,
    };
    RunReport {
        files: vec!["source.txt".into()],
        diagnostics: vec![finding.clone(), finding],
    }
}
