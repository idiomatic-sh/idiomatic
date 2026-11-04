//! Request validation rejects unknown and missing fields without exposing wire types.
use super::{WorkerSchemaError, request};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::ffi::OsStr;
type Input = &'static str;
type Value = bool;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    InvalidJson,
    Unexpected,
}
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unicode source path is preserved"),
        word: Word::Succeeds,
        input: || r#"{"root":"root","rules":"rules","files":["é.txt"],"cache":null}"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("explicit query module registry is preserved"),
        word: Word::Succeeds,
        input: || r#"{"root":"root","rules":"rules","query_modules":{"python/ruff/suppress":"fragments/suppress.ryoql"},"files":["source.py"],"cache":null}"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown request fields are rejected"),
        word: Word::Refuses,
        input: || r#"{"root":"root","rules":"rules","files":[],"extra":true}"#,
        expect: Expect::Error(Refusal::InvalidJson),
    },
    Case {
        name: CaseName("required request fields cannot be omitted"),
        word: Word::Refuses,
        input: || r#"{"root":"root","rules":"rules"}"#,
        expect: Expect::Error(Refusal::InvalidJson),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    request(OsStr::new(input))
        .map(|request| {
            let registry_matches = if input.contains("query_modules") {
                request
                    .query_modules
                    .get("python/ruff/suppress")
                    .is_some_and(|path| path == std::path::Path::new("fragments/suppress.ryoql"))
            } else {
                request.query_modules.is_empty()
            };
            registry_matches
                && (request.files == [ryo::linter::RelativePath("é.txt".into())]
                    || request.files == [ryo::linter::RelativePath("source.py".into())])
                && matches!(
                    request.state,
                    crate::domain::suite::types::WorkerState::Fresh
                )
        })
        .map_err(|error| match error {
            WorkerSchemaError::Json(_) => Refusal::InvalidJson,
            WorkerSchemaError::NotText
            | WorkerSchemaError::InvalidRule(_)
            | WorkerSchemaError::Observation(_)
            | WorkerSchemaError::Configuration(_) => Refusal::Unexpected,
        })
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
fn explicit_cohort_reaches_the_worker_without_changing_legacy_absence() {
    let legacy = request(OsStr::new(
        r#"{"root":"root","rules":"rules","files":["source.py"],"cache":null}"#,
    ))
    .unwrap();
    assert!(legacy.selected_rules.is_none());
    let selected = request(OsStr::new(r#"{"root":"root","rules":"rules","files":["source.py"],"cache":null,"selected_rules":["F401","RUF100"]}"#)).unwrap();
    assert_eq!(
        selected
            .selected_rules
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["F401", "RUF100"]
    );
}

#[test]
fn configured_context_preserves_exact_data_and_rejects_incomplete_shapes() {
    let value = serde_json::json!({"root":"root","rules":"rules","files":["é.py"],
    "selected_rules":["F401","RUF100"], "rule_context": {
        "tables":{"selected":"chosen","disabled":"disabled","external":"external"},
        "configuration":{"external-prefixes":["V101"],"disabled-rules":{"é.py":["F401"]}}
    }});
    let decoded = request(OsStr::new(&value.to_string())).unwrap();
    let prepared = decoded.rule_context.unwrap();
    assert_eq!(prepared.tables.selected, "chosen");
    assert_eq!(prepared.configuration.external_prefixes, ["V101"]);
    assert_eq!(
        crate::domain::suite::rule_context::selected_pairs(
            &prepared,
            decoded.selected_rules.as_deref().unwrap(),
            &decoded.files,
        )
        .unwrap(),
        1
    );
    for key in ["external-prefixes", "disabled-rules"] {
        let mut missing = value.clone();
        missing["rule_context"]["configuration"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(request(OsStr::new(&missing.to_string())).is_err());
    }
    let mut extra = value;
    extra["rule_context"]["configuration"]["implicit"] = true.into();
    assert!(request(OsStr::new(&extra.to_string())).is_err());
}
