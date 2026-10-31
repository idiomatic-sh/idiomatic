//! Real SDK evidence for missing files, unread inputs and typed findings.
use super::{BoundaryFailure, failure};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::domain::{
    diagnostics::{errors::DiagnosticsError as Error, types::BenchmarkFinding},
    suite::types::WorkerRequest,
};
use ryo_benchmark::outbound::engine::{factory, interface::EnginePort};

#[derive(Clone, Copy)]
enum EngineScenario {
    Finding,
    Missing,
    Unread,
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
        name: CaseName("unread_input_is_not_checked"),
        word: Word::Refuses,
        input: || EngineScenario::Unread,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("missing_listed_file_is_failure"),
        word: Word::Fails,
        input: || EngineScenario::Missing,
        expect: Expect::Error(BoundaryFailure::Io(std::io::ErrorKind::NotFound)),
    },
];
fn exercise(scenario: Input) -> Result<Value, Error> {
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    std::fs::create_dir_all(rules.join("E722"))?;
    std::fs::write(
        rules.join("E722/query.ryoql"),
        include_str!("rules/E722/query.ryoql"),
    )?;
    let file = match scenario {
        EngineScenario::Finding => "input.py",
        EngineScenario::Missing => "missing.py",
        EngineScenario::Unread => "input.unknown",
    };
    if !matches!(scenario, EngineScenario::Missing) {
        std::fs::write(
            root.path().join(file),
            "try:\n    pass\nexcept:\n    pass\n",
        )?;
    }
    let request = WorkerRequest {
        root: root.path().to_owned(),
        rules,
        files: vec![ryo::linter::RelativePath(file.into())],
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
    };
    let result =
        factory::local(ryo::linter::Languages(vec![ryo_python::linked()]), None).run(request);
    if matches!(scenario, EngineScenario::Unread) {
        assert!(
            matches!(&result, Err(Error::Invalid(message)) if message.starts_with("incomplete candidate:") && message.contains("input.unknown"))
        );
    }
    let report = result?;
    Ok(
        report.files == [ryo::linter::RelativePath("input.py".into())]
            && report.diagnostics
                == [BenchmarkFinding {
                    rule: ryo::linter::RuleId::try_from("E722").expect("fixture rule"),
                    file: ryo::linter::RelativePath("input.py".into()),
                    start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(14),
                    end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(20),
                }],
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
