//! The shared preparation boundary rejects physical drift before linter preparation.
use super::*;
use crate::domain::{
    diagnostics::types::{ExecutionMetadata, RunReport},
    measurements::types::{Invocation, ProcessOutput},
    suite::types::{CaseComparison, PreparedCase, RunRequest},
};
use crate::outbound::{execution::fixtures::fake::ExecutionFake, linter::interface::LinterPort};
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
                performance: Default::default(),
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
            &crate::outbound::suite::factory::local(),
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
            super::candidate(
                &crate::outbound::suite::factory::local(),
                &candidate,
                &execution,
                &case.request
            )
            .is_err()
        );
        assert_eq!(candidate.0.get(), 0);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        assert!(
            super::candidate(
                &crate::outbound::suite::factory::local(),
                &candidate,
                &execution,
                &case.request
            )
            .unwrap_err()
            .to_string()
            .contains("preparation reached")
        );
        assert_eq!(candidate.0.get(), 1);
        assert!(execution.history.lock().unwrap().is_empty());
    }
}

#[test]
fn declared_missing_primary_is_reverified_before_both_contestants_prepare() {
    use crate::domain::suite::types::CaseContext;
    let scratch = tempfile::tempdir().unwrap();
    let boundary = scratch.path().join("snapshot");
    let sources = [(
        ryo::linter::RelativePath("companion.py".into()),
        ryo::linter::SourceText("x = 1\n".into()),
    )]
    .into();
    let manifest = crate::outbound::context::adapters::local::adapter::materialize(
        &boundary,
        &CaseContext {
            logical_root: "project".into(),
            auxiliary_files: Default::default(),
            missing: vec!["project/gone.py".into()],
        },
        &sources,
    )
    .unwrap();
    let root = boundary.join("project");
    let files = vec![
        ryo::linter::RelativePath("companion.py".into()),
        ryo::linter::RelativePath("gone.py".into()),
    ];
    let case = PreparedCase {
        comparison: CaseComparison::Full,
        rule: ryo::linter::RuleId::try_from("R1").unwrap(),
        name: "missing".into(),
        request: RunRequest {
            file_metadata: None,
            context_manifest: Some(manifest),
            root: root.clone(),
            files: files.clone(),
            state: scratch.path().join("cache"),
        },
        expected: RunReport {
            execution_metadata: ExecutionMetadata::Complete,
            project_context: None,
            diagnostics: Vec::new(),
            skipped: Vec::new(),
            coverage: None,
            files,
        },
        reference_discrepancy: None,
    };
    let execution = ExecutionFake::default();
    let observed = ObservedPrepare(Cell::new(0));
    for appeared in [false, true, false] {
        let path = root.join("gone.py");
        if appeared {
            std::fs::write(&path, "now present\n").unwrap();
        } else if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        let before = observed.0.get();
        let candidate = super::candidate(
            &crate::outbound::suite::factory::local(),
            &observed,
            &execution,
            &case.request,
        );
        let reference = run(
            &crate::outbound::suite::factory::local(),
            &observed,
            &execution,
            &case,
            scratch.path(),
        )
        .result;
        for result in [candidate, reference] {
            let message = result.unwrap_err().to_string();
            assert_eq!(message.contains("preparation reached"), !appeared);
        }
        assert_eq!(observed.0.get() - before, if appeared { 0 } else { 2 });
    }
    assert!(execution.history.lock().unwrap().is_empty());
}
