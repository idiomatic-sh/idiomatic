//! The shared preparation boundary rejects physical drift before linter preparation.
use super::*;
use crate::domain::{
    diagnostics::types::{ExecutionMetadata, RunReport},
    measurements::types::{Invocation, ProcessOutput},
    suite::types::{CaseComparison, PreparedCase, RunRequest},
};
use crate::outbound::{
    execution::fixtures::fake::ExecutionFake, linter::interface::LinterPort,
    suite::adapters::local::adapter::SuiteLocalAdapter,
};
use std::{cell::Cell, collections::BTreeMap};
struct ObservedPrepare(Cell<usize>);
impl LinterPort for ObservedPrepare {
    fn prepare(&self, _: &RunRequest) -> Result<Invocation, Error> {
        self.0.set(self.0.get() + 1);
        Err(Error::Invalid("preparation reached".into()))
    }
    fn decode(&self, _: &RunRequest, _: ProcessOutput) -> Result<RunReport, Error> {
        unreachable!("this admission test stops at prepare")
    }
}
#[cfg(unix)]
#[test]
fn metadata_drift_blocks_each_attempt_before_prepare_with_a_live_positive_control() {
    use std::os::unix::fs::PermissionsExt as _;
    for expected in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("input.txt");
        std::fs::write(&path, b"same bytes").unwrap();
        let mode = if expected { 0o744 } else { 0o644 };
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        let file = ryo::linter::RelativePath("input.txt".into());
        let case = PreparedCase {
            comparison: CaseComparison::Full,
            rule: ryo::linter::RuleId::try_from("A001").unwrap(),
            name: "mode".into(),
            request: RunRequest {
                file_metadata: Some(BTreeMap::from([(file.clone(), expected)])),
                context_manifest: None,
                root: root.path().to_path_buf(),
                files: vec![file.clone()],
                state: root.path().join("cache"),
            },
            expected: RunReport {
                execution_metadata: ExecutionMetadata::Complete,
                project_context: None,
                diagnostics: Vec::new(),
                skipped: Vec::new(),
                coverage: None,
                files: vec![file],
            },
            reference_discrepancy: None,
        };
        let execution = ExecutionFake::default();
        let reference = ObservedPrepare(Cell::new(0));
        let result = run(
            &SuiteLocalAdapter,
            &reference,
            &execution,
            &case,
            root.path(),
        );
        assert!(
            result
                .result
                .unwrap_err()
                .to_string()
                .contains("preparation reached")
        );
        assert_eq!(reference.0.get(), 1);
        std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(if expected { 0o644 } else { 0o744 }),
        )
        .unwrap();
        let candidate = ObservedPrepare(Cell::new(0));
        assert!(
            super::candidate(&SuiteLocalAdapter, &candidate, &execution, &case.request).is_err()
        );
        assert_eq!(candidate.0.get(), 0);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        assert!(
            super::candidate(&SuiteLocalAdapter, &candidate, &execution, &case.request)
                .unwrap_err()
                .to_string()
                .contains("preparation reached")
        );
        assert_eq!(candidate.0.get(), 1);
        assert!(execution.history.lock().unwrap().is_empty());
    }
}
