//! Real SDK evidence for missing files, unread inputs and typed findings.
use ryo_benchmark::domain::{
    diagnostics::{errors::Error, types::Diagnostic},
    suite::types::WorkerRequest,
};
use ryo_benchmark::outbound::engine::{factory, interface::EnginePort};

fn rules(directory: &std::path::Path) -> std::path::PathBuf {
    let root = directory.join("rules");
    std::fs::create_dir_all(root.join("E722")).unwrap();
    std::fs::write(
        root.join("E722/query.ryoql"),
        include_str!("rules/E722/query.ryoql"),
    )
    .unwrap();
    root
}

#[test]
fn missing_listed_file_is_a_failure() {
    let root = tempfile::tempdir().unwrap();
    let request = WorkerRequest {
        root: root.path().to_owned(),
        rules: rules(root.path()),
        files: vec!["missing.py".into()],
        cache: None,
    };
    let error = factory::local(ryo::linter::Languages(vec![ryo_python::linked()]))
        .run(request)
        .unwrap_err();
    assert!(matches!(error, Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound));
}

#[test]
fn unread_input_is_not_reported_as_checked() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("input.unknown"), "x").unwrap();
    let request = WorkerRequest {
        root: root.path().to_owned(),
        rules: rules(root.path()),
        files: vec!["input.unknown".into()],
        cache: None,
    };
    let error = factory::local(ryo::linter::Languages(vec![ryo_python::linked()]))
        .run(request)
        .unwrap_err();
    assert!(
        matches!(error, Error::Invalid(message) if message.starts_with("incomplete candidate:") && message.contains("input.unknown"))
    );
}

#[test]
fn sdk_findings_preserve_rule_file_and_byte_span() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("input.py"),
        "try:\n    pass\nexcept:\n    pass\n",
    )
    .unwrap();
    let request = WorkerRequest {
        root: root.path().to_owned(),
        rules: rules(root.path()),
        files: vec!["input.py".into()],
        cache: None,
    };
    let report = factory::local(ryo::linter::Languages(vec![ryo_python::linked()]))
        .run(request)
        .unwrap();
    assert_eq!(report.files, ["input.py"]);
    assert_eq!(
        report.diagnostics,
        [Diagnostic {
            rule: "E722".into(),
            file: "input.py".into(),
            start: 14,
            end: 20
        }]
    );
}
