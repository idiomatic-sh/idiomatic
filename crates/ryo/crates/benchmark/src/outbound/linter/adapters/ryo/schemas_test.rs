//! Wire reports preserve the canonical fields and reject unknown or malformed input.
use super::decode_report;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::types::{BenchmarkFinding, RunReport};
type Input = &'static [u8];
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("blank rule identity is refused"),
        word: Word::Refuses,
        input: || {
            br#"{"diagnostics":[{"rule":"","file":"input","start":0,"end":1}],"files":["input"]}"#
        },
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("all canonical fields survive"),
        word: Word::Succeeds,
        input: || {
            br#"{"diagnostics":[{"rule":"R1","file":"input","start":2,"end":7}],"files":["input","clean"]}"#
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown fields are refused"),
        word: Word::Refuses,
        input: || br#"{"diagnostics":[],"files":[],"extra":true}"#,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("invalid json is refused"),
        word: Word::Refuses,
        input: || b"not-json",
        expect: Expect::Error(()),
    },
];
fn subject(bytes: Input) -> Result<Value, Failure> {
    decode_report(bytes)
        .map(|report| {
            report
                == RunReport {
                    diagnostics: vec![BenchmarkFinding {
                        rule: ryo::linter::RuleId::try_from("R1").expect("fixture rule"),
                        file: ryo::linter::RelativePath("input".into()),
                        start: crate::domain::diagnostics::types::SourceByteOffset(2),
                        end: crate::domain::diagnostics::types::SourceByteOffset(7),
                    }],
                    files: vec![
                        ryo::linter::RelativePath("input".into()),
                        ryo::linter::RelativePath("clean".into()),
                    ],
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

#[test]
fn request_transports_the_explicit_module_registry() {
    let mut request = crate::outbound::worker::fixtures::data::request();
    request.query_modules.insert(
        "python/ruff/suppress".into(),
        "fragments/suppress.ryoql".into(),
    );
    let encoded = super::encode_request(request).expect("request JSON");
    let value: serde_json::Value = serde_json::from_str(&encoded).expect("valid request JSON");
    assert_eq!(
        value["query_modules"]["python/ruff/suppress"],
        "fragments/suppress.ryoql"
    );
}
