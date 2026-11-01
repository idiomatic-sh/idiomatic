//! Real SDK evidence for missing files, unread or partial inputs and typed findings.
use super::{BoundaryFailure, failure};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::domain::{
    diagnostics::{errors::DiagnosticsError as Error, types::BenchmarkFinding},
    suite::types::WorkerRequest,
};
use ryo_benchmark::outbound::engine::{factory, interface::EnginePort as _};

#[derive(Clone, Copy)]
enum EngineScenario {
    Finding,
    Missing,
    Unread,
    Partial,
    InvalidSyntax,
    InvalidSyntaxProjectionDisabled,
}
type Input = EngineScenario;
type Value = bool;
type Failure = BoundaryFailure;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("sdk_preserves_checked_file_rule_and_byte_span"),
        word: Word::Succeeds,
        input: || EngineScenario::Finding,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("sdk_projects_module_nonlocal_as_invalid_syntax"),
        word: Word::Succeeds,
        input: || EngineScenario::InvalidSyntax,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("frontend_projection_is_disabled_without_an_identity"),
        word: Word::Succeeds,
        input: || EngineScenario::InvalidSyntaxProjectionDisabled,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unread_input_is_not_checked"),
        word: Word::Refuses,
        input: || EngineScenario::Unread,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("partial_relations_cannot_be_reported_as_checked"),
        word: Word::Refuses,
        input: || EngineScenario::Partial,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("missing_listed_file_is_failure"),
        word: Word::Fails,
        input: || EngineScenario::Missing,
        expect: Expect::Error(BoundaryFailure::Io(std::io::ErrorKind::NotFound)),
    },
];
#[expect(
    clippy::expect_used,
    reason = "the fixed fixture rule identifier must be valid for engine assertions"
)]
fn exercise(scenario: Input) -> Result<Value, Error> {
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    std::fs::create_dir_all(rules.join("E722"))?;
    std::fs::write(
        rules.join("E722/query.ryoql"),
        include_str!("rules/E722/query.ryoql"),
    )?;
    let file = match scenario {
        EngineScenario::Finding
        | EngineScenario::Partial
        | EngineScenario::InvalidSyntax
        | EngineScenario::InvalidSyntaxProjectionDisabled => "input.py",
        EngineScenario::Missing => "missing.py",
        EngineScenario::Unread => "input.unknown",
    };
    if !matches!(scenario, EngineScenario::Missing) {
        std::fs::write(
            root.path().join(file),
            if matches!(scenario, EngineScenario::Partial) {
                "try:\n    pass\nexcept:\n    pass\n\ndef broken(\n"
            } else if matches!(
                scenario,
                EngineScenario::InvalidSyntax | EngineScenario::InvalidSyntaxProjectionDisabled
            ) {
                "global x\nnonlocal x\n"
            } else {
                "try:\n    pass\nexcept:\n    pass\n"
            },
        )?;
    }
    let request = WorkerRequest {
        root: root.path().to_owned(),
        rules,
        files: vec![ryo::linter::RelativePath(file.into())],
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
    };
    let result = if matches!(scenario, EngineScenario::InvalidSyntaxProjectionDisabled) {
        factory::local(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::PythonParserSettings::new(ryo_python::PythonTargetVersion::Py310),
            )]),
            None,
        )
        .run(request)
    } else {
        factory::local_with_frontend_diagnostic(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::PythonParserSettings::new(ryo_python::PythonTargetVersion::Py310),
            )]),
            None,
            ryo::linter::RuleId::try_from("invalid-syntax").expect("frontend identity"),
        )
        .run(request)
    };
    if matches!(scenario, EngineScenario::Unread) {
        assert!(
            matches!(&result, Err(Error::Invalid(message)) if message.starts_with("incomplete candidate:") && message.contains("input.unknown")),
            "unread input must be reported as incomplete candidate coverage"
        );
    }
    if matches!(scenario, EngineScenario::Partial) {
        assert!(
            matches!(&result, Err(Error::Invalid(message))
                if message.starts_with("incomplete candidate:")
                    && !message.contains("unknowns=[]")
                    && message.contains("unread=[]")),
            "partial relation availability must refuse despite complete file coverage"
        );
    }
    let report = result?;
    let expected_diagnostics = if matches!(scenario, EngineScenario::InvalidSyntax) {
        vec![BenchmarkFinding {
            rule: ryo::linter::RuleId::try_from("invalid-syntax").expect("syntax rule"),
            file: ryo::linter::RelativePath("input.py".into()),
            start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(9),
            end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(19),
        }]
    } else if matches!(scenario, EngineScenario::InvalidSyntaxProjectionDisabled) {
        vec![]
    } else {
        vec![BenchmarkFinding {
            rule: ryo::linter::RuleId::try_from("E722").expect("fixture rule"),
            file: ryo::linter::RelativePath("input.py".into()),
            start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(14),
            end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(20),
        }]
    };
    Ok(
        report.files == [ryo::linter::RelativePath("input.py".into())]
            && report.diagnostics == expected_diagnostics,
    )
}
fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input).map_err(failure)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
