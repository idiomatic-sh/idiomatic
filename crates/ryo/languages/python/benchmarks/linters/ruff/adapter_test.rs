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
