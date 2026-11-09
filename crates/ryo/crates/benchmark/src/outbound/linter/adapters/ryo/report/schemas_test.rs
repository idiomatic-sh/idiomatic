//! Wire reports preserve the canonical fields and reject unknown or malformed input.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::linter::adapters::ryo::types::ReportBytes;
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use std::borrow::Cow;
type Input = Cow<'static, [u8]>;
fn report(bytes: &'static [u8]) -> Input {
    Cow::Borrowed(bytes)
}
/// A decoded-report wire missing a required key, or with a coverage counter that cannot be zero.
fn invalid_report(removed: Option<&str>, counter: Option<&str>) -> Input {
    let mut report = serde_json::json!({
        "execution_metadata": {"availability": "complete"},
        "diagnostics": [], "files": ["bad.py"], "skipped": [],
        "coverage": {"selected": 1, "evaluated": 1, "skipped": 0, "unknown_or_unread": 0},
    });
    if let Some(path) = removed {
        let _removed = report.as_object_mut().expect("fixture object").remove(path);
    }
    if let Some(field) = counter {
        report["coverage"][field] = 1.into();
    }
    Cow::Owned(serde_json::to_vec(&report).expect("fixture JSON"))
}
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("blank rule identity is refused"),
        word: Word::Refuses,
        input: || {
            report(br#"{"diagnostics":[{"rule":"","file":"input","start":0,"end":1}],"files":["input"]}"#)
        },
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("all canonical fields survive"),
        word: Word::Succeeds,
        input: || {
            report(br#"{"diagnostics":[{"rule":"R1","file":"input","start":2,"end":7}],"files":["input","clean"],"execution_metadata":{"availability":"complete"},"coverage":{"selected":2,"evaluated":2,"skipped":0,"unknown_or_unread":0}}"#)
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("typed syntax rejection remains distinct from an empty result"),
        word: Word::Succeeds,
        input: || {
            report(br#"{"diagnostics":[],"files":["bad.py"],"skipped":[{"rule":"F401","path":"bad.py","reason":"syntax-rejected"}],"execution_metadata":{"availability":"complete"},"coverage":{"selected":1,"evaluated":0,"skipped":1,"unknown_or_unread":0}}"#)
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown skip reasons are refused"),
        word: Word::Refuses,
        input: || {
            report(br#"{"diagnostics":[],"files":["bad.py"],"skipped":[{"rule":"F401","path":"bad.py","reason":"unknown"}]}"#)
        },
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("unknown fields are refused"),
        word: Word::Refuses,
        input: || report(br#"{"diagnostics":[],"files":[],"extra":true}"#),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("invalid json is refused"),
        word: Word::Refuses,
        input: || report(b"not-json"),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("complete metadata removed is refused"),
        word: Word::Refuses,
        input: || invalid_report(Some("execution_metadata"), None),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("coverage removed is refused"),
        word: Word::Refuses,
        input: || invalid_report(Some("coverage"), None),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("unknown or unread coverage cannot be known zero"),
        word: Word::Refuses,
        input: || invalid_report(None, Some("unknown_or_unread")),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("skipped coverage without skipped entries is refused"),
        word: Word::Refuses,
        input: || invalid_report(None, Some("skipped")),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("a receipt of an unknown project context mode is refused"),
        word: Word::Refuses,
        input: || {
            report(br#"{"diagnostics":[],"files":[],"project_context":{"mode":"disk-by-default","logical_root":"pkg","actual_root":"/snapshot/pkg","manifest_hash":"context-hash","dependencies":{}}}"#)
        },
        expect: Expect::Error(()),
    },
];
fn subject(wire: Input) -> Result<Value, Failure> {
    let bytes = wire;
    RunReport::try_from(ReportBytes(&bytes))
        .map(|report| {
            if !report.skipped.is_empty() {
                return report.diagnostics.is_empty()
                    && report.skipped.len() == 1
                    && report.skipped[0].rule.as_ref() == "F401"
                    && report.skipped[0].path.0.as_ref() == "bad.py"
                    && report.skipped[0].reason == ryo::linter::SkipReason::SyntaxRejected;
            }
            report == RunReport {
                execution_metadata:
                    ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                project_context:
                    ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                diagnostics: vec![BenchmarkFinding {
                    rule: ryo::linter::RuleId::try_from("R1").expect("fixture rule"),
                    file: ryo::linter::RelativePath("input".into()),
                    start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(2),
                    end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(7),
                }],
                files: vec![
                    ryo::linter::RelativePath("input".into()),
                    ryo::linter::RelativePath("clean".into()),
                ],
                skipped: Vec::new(),
                coverage:
                    ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
                        ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                            selected: ryo_benchmark_contracts::domain::reports::types::PairCount(2),
                            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(
                                2,
                            ),
                            skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                            unknown_or_unread:
                                ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                        },
                    ),
            }
        })
        .map_err(|_rejected_report| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
