//! Operational failure and accidental directory walks never count as clean runs.
use super::adapter::Ruff;
use ryo_benchmark::domain::measurements::types::ProcessOutput;
use ryo_benchmark::domain::suite::types::RunRequest;
use ryo_benchmark::outbound::linter::interface::LinterPort;
#[test]
fn operational_error_is_not_a_finding() {
    let ruff = Ruff {
        executable: "ruff".into(),
        defaults: Default::default(),
    };
    let request = RunRequest {
        root: "/tmp".into(),
        files: vec!["a.py".into()],
        cache: "/tmp/cache".into(),
    };
    assert!(
        ruff.decode(
            &request,
            ProcessOutput {
                status: 2,
                stdout: b"[]".to_vec(),
                stderr: vec![],
                wall: Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn empty_files_cannot_walk() {
    let ruff = Ruff {
        executable: "ruff".into(),
        defaults: Default::default(),
    };
    let request = RunRequest {
        root: "/tmp".into(),
        files: vec![],
        cache: "/tmp/cache".into(),
    };
    assert!(ruff.prepare(&request).is_err());
}
#[test]
fn invocation_is_isolated_without_selection_override() {
    let ruff = Ruff {
        executable: "ruff".into(),
        defaults: Default::default(),
    };
    let request = RunRequest {
        root: "/tmp".into(),
        files: vec!["a.py".into()],
        cache: "/tmp/cache".into(),
    };
    let invocation = ruff.prepare(&request).unwrap();
    let args: Vec<_> = invocation
        .arguments
        .iter()
        .map(|arg| arg.to_str().unwrap())
        .collect();
    assert_eq!(
        args,
        [
            "check",
            "--isolated",
            "--output-format",
            "json",
            "--cache-dir",
            "/tmp/cache/ruff",
            "--",
            "a.py"
        ]
    );
}

#[test]
fn discovery_failure_is_not_empty_success() {
    let ruff = Ruff {
        executable: "/definitely/missing/ruff".into(),
        defaults: Default::default(),
    };
    assert!(ruff.files(std::path::Path::new("/tmp")).is_err());
}

#[test]
fn valid_lint_json_with_a_warning_is_not_an_operational_failure() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("case.py");
    std::fs::write(&path, "try:\n    pass\nexcept: # ruff: noqa\n    pass\n").unwrap();
    let ruff = Ruff {
        executable: "ruff".into(),
        defaults: ["E722".into()].into_iter().collect(),
    };
    let request = RunRequest {
        root: root.path().to_owned(),
        files: vec!["case.py".into()],
        cache: root.path().join("cache"),
    };
    let stdout = serde_json::to_vec(&serde_json::json!([{"code":"E722","filename":path,"cell":null,"location":{"row":3,"column":1},"end_location":{"row":3,"column":7}}])).unwrap();
    let report = ruff
        .decode(
            &request,
            ProcessOutput {
                status: 1,
                stdout,
                stderr: b"warning: file-level noqa must be on its own line\n".to_vec(),
                wall: Default::default(),
            },
        )
        .unwrap();
    assert_eq!(
        report.diagnostics,
        [ryo_benchmark::domain::diagnostics::types::Diagnostic {
            rule: "E722".into(),
            file: "case.py".into(),
            start: 14,
            end: 20
        }]
    );
}
