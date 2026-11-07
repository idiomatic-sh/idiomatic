//! Wire reports preserve the canonical fields and reject unknown or malformed input.
use super::decode_report;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::borrow::Cow;
/// What a row feeds the schemas: a report to decode, or a request to encode and inspect.
enum Wire {
    Report(Cow<'static, [u8]>),
    Request(fn() -> WorkerRequest, fn(&serde_json::Value) -> bool),
}
type Input = Wire;
fn report(bytes: &'static [u8]) -> Input {
    Wire::Report(Cow::Borrowed(bytes))
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
    Wire::Report(Cow::Owned(
        serde_json::to_vec(&report).expect("fixture JSON"),
    ))
}
fn module_registry_request() -> WorkerRequest {
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
    request
}
fn module_registry_is_transported(value: &serde_json::Value) -> bool {
    value["query_modules"]["python/ruff/suppress"] == "fragments/suppress.ryoql"
        && value["rule_applicability"]["F401"] == "accepted-syntax"
}
fn absent_cohort_request() -> WorkerRequest {
    // The external wire can omit selection; worker preparation still requires resolution.
    let mut request = crate::outbound::worker::fixtures::data::request();
    request.selected_rules = None;
    request
}
fn legacy_wire_shape(value: &serde_json::Value) -> bool {
    value.get("selected_rules").is_none()
}
fn explicit_cohort_request() -> WorkerRequest {
    let mut request = crate::outbound::worker::fixtures::data::request();
    request.selected_rules = Some(vec![
        ryo::linter::RuleId::try_from("F401").expect("fixture rule"),
        ryo::linter::RuleId::try_from("RUF100").expect("fixture rule"),
    ]);
    request
}
fn explicit_cohort_is_serialized(value: &serde_json::Value) -> bool {
    value["selected_rules"] == serde_json::json!(["F401", "RUF100"])
}
fn listed_context_request() -> WorkerRequest {
    let mut request = crate::outbound::worker::fixtures::data::request();
    request.project_context_mode =
        ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::ListedDisk;
    request.context_manifest = Some(ryo_benchmark_contracts::domain::context::types::ContextManifest {
        boundary: "/snapshot".into(),
        logical_root: "pkg".into(),
        auxiliary_files: std::collections::BTreeMap::from([(
            "pkg/__init__.py".into(),
            ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest(
                "auxiliary-hash".into(),
            ),
        )]),
        source_files: std::collections::BTreeMap::new(),
        missing: vec!["__init__.py".into()],
    });
    request
}
fn listed_context_is_transported(value: &serde_json::Value) -> bool {
    value["project_context_mode"] == "listed-disk"
        && value["context_manifest"]["logical_root"] == "pkg"
        && value["context_manifest"]["auxiliary_files"]["pkg/__init__.py"] == "auxiliary-hash"
        && value["context_manifest"]["missing"] == serde_json::json!(["__init__.py"])
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
        name: CaseName("request transports the explicit module registry"),
        word: Word::Succeeds,
        input: || Wire::Request(module_registry_request, module_registry_is_transported),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("absent cohort keeps the legacy wire shape"),
        word: Word::Succeeds,
        input: || Wire::Request(absent_cohort_request, legacy_wire_shape),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("explicit cohort is serialized"),
        word: Word::Succeeds,
        input: || Wire::Request(explicit_cohort_request, explicit_cohort_is_serialized),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("request transports the sealed context manifest and its mode"),
        word: Word::Succeeds,
        input: || Wire::Request(listed_context_request, listed_context_is_transported),
        expect: Expect::Value(true),
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
    let bytes = match wire {
        Wire::Report(bytes) => bytes,
        Wire::Request(build, check) => {
            let encoded = super::encode_request(build()).map_err(|_rejected_request| ())?;
            let value: serde_json::Value =
                serde_json::from_str(&encoded).map_err(|_invalid_json| ())?;
            return Ok(check(&value));
        }
    };
    decode_report(&bytes)
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
                        ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                    project_context: None,
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
                    coverage: Some(ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
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
