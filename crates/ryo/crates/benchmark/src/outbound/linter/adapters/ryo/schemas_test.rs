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
            br#"{"diagnostics":[{"rule":"R1","file":"input","start":2,"end":7}],"files":["input","clean"],"execution_metadata":{"availability":"complete"},"coverage":{"selected":2,"evaluated":2,"skipped":0,"unknown_or_unread":0}}"#
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("typed syntax rejection remains distinct from an empty result"),
        word: Word::Succeeds,
        input: || {
            br#"{"diagnostics":[],"files":["bad.py"],"skipped":[{"rule":"F401","path":"bad.py","reason":"syntax-rejected"}],"execution_metadata":{"availability":"complete"},"coverage":{"selected":1,"evaluated":0,"skipped":1,"unknown_or_unread":0}}"#
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown skip reasons are refused"),
        word: Word::Refuses,
        input: || {
            br#"{"diagnostics":[],"files":["bad.py"],"skipped":[{"rule":"F401","path":"bad.py","reason":"unknown"}]}"#
        },
        expect: Expect::Error(()),
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
            if !report.skipped.is_empty() {
                return report.diagnostics.is_empty()
                    && report.skipped.len() == 1
                    && report.skipped[0].rule.as_ref() == "F401"
                    && report.skipped[0].path.0.as_ref() == "bad.py"
                    && report.skipped[0].reason == ryo::linter::SkipReason::SyntaxRejected;
            }
            report
                == RunReport {
                    execution_metadata:
                        crate::domain::diagnostics::types::ExecutionMetadata::Complete,
                    project_context: None,
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
                    skipped: Vec::new(),
                    coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
                        selected: 2,
                        evaluated: 2,
                        skipped: 0,
                        unknown_or_unread: 0,
                    }),
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
fn legacy_or_incomplete_candidate_metadata_never_becomes_known_zero() {
    let report = serde_json::json!({
        "execution_metadata": {"availability": "complete"},
        "diagnostics": [], "files": ["bad.py"], "skipped": [],
        "coverage": {"selected": 1, "evaluated": 1, "skipped": 0, "unknown_or_unread": 0},
    });
    for path in ["execution_metadata", "coverage"] {
        let mut invalid = report.clone();
        let _removed = invalid
            .as_object_mut()
            .expect("fixture object")
            .remove(path);
        assert!(decode_report(&serde_json::to_vec(&invalid).expect("fixture JSON")).is_err());
    }
    for field in ["unknown_or_unread", "skipped"] {
        let mut invalid = report.clone();
        invalid["coverage"][field] = 1.into();
        assert!(decode_report(&serde_json::to_vec(&invalid).expect("fixture JSON")).is_err());
    }
}

#[test]
fn request_transports_the_explicit_module_registry() {
    let mut request = crate::outbound::worker::fixtures::data::request();
    assert!(
        request
            .query_modules
            .insert(
                "python/ruff/suppress".into(),
                "fragments/suppress.ryoql".into(),
            )
            .is_none()
    );
    let _previous = request
        .rule_applicability
        .insert("F401".into(), "accepted-syntax".into());
    let encoded = super::encode_request(request).expect("request JSON");
    let value: serde_json::Value = serde_json::from_str(&encoded).expect("valid request JSON");
    assert_eq!(
        value["query_modules"]["python/ruff/suppress"],
        "fragments/suppress.ryoql"
    );
    assert_eq!(value["rule_applicability"]["F401"], "accepted-syntax");
}

#[test]
fn explicit_cohort_is_serialized_but_absent_cohort_keeps_the_legacy_wire_shape() {
    let mut request = crate::outbound::worker::fixtures::data::request();
    // The external wire can omit selection; worker preparation still requires resolution.
    request.selected_rules = None;
    let encoded = super::encode_request(request.clone()).unwrap();
    let legacy: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert!(legacy.get("selected_rules").is_none());
    request.selected_rules = Some(vec![
        ryo::linter::RuleId::try_from("F401").unwrap(),
        ryo::linter::RuleId::try_from("RUF100").unwrap(),
    ]);
    let encoded = super::encode_request(request).unwrap();
    let explicit: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        explicit["selected_rules"],
        serde_json::json!(["F401", "RUF100"])
    );
}
