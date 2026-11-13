//! Request validation rejects unknown and missing fields without exposing wire types.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::worker::adapters::json::types::WorkerPayload;
use ryo_benchmark_contracts::domain::context::types::{ExternalPrefix, TableName};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, QueryModuleName, RuleContextPreparation, WorkerRequest, WorkerState,
};
use std::borrow::Cow;
use std::ffi::OsStr;
/// What the decoded request must show; a refused wire has none.
type Check = Option<fn(&WorkerRequest) -> bool>;
type Input = (Cow<'static, str>, Check);
type Value = bool;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    InvalidJson,
    Unexpected,
}
type Failure = Refusal;

fn fresh_with_files(request: &WorkerRequest, file: &str) -> bool {
    request.files == [ryo::linter::RelativePath(file.into())]
        && matches!(request.state, WorkerState::Fresh)
}
fn unicode_source_path(request: &WorkerRequest) -> bool {
    request.query_modules.is_empty() && fresh_with_files(request, "é.txt")
}
fn explicit_query_module_registry(request: &WorkerRequest) -> bool {
    request
        .query_modules
        .get(&QueryModuleName("python/ruff/suppress".into()))
        .is_some_and(|path| path == std::path::Path::new("fragments/suppress.ryoql"))
        && fresh_with_files(request, "source.py")
}
fn legacy_absence(request: &WorkerRequest) -> bool {
    matches!(request.selected_rules, CohortSelection::Frozen)
}
fn explicit_cohort(request: &WorkerRequest) -> bool {
    matches!(&request.selected_rules, CohortSelection::Explicit(rules)
        if rules.iter().map(ToString::to_string).collect::<Vec<_>>() == ["F401", "RUF100"])
}
fn configured_context(request: &WorkerRequest) -> bool {
    let (RuleContextPreparation::Prepared(prepared), CohortSelection::Explicit(selected)) =
        (&request.rule_context, &request.selected_rules)
    else {
        return false;
    };
    let disabled: usize = prepared
        .configuration
        .disabled_rules
        .values()
        .map(Vec::len)
        .sum();
    prepared.tables.selected == TableName("chosen".into())
        && prepared.configuration.external_prefixes == [ExternalPrefix("V101".into())]
        && (selected.len() * request.files.len()).checked_sub(disabled) == Some(1)
}

/// The configured-context wire, with one configuration key removed or an undeclared one added.
fn context_wire(removed: Option<&str>, implicit: bool) -> Cow<'static, str> {
    let mut value = serde_json::json!({"root":"root","rules":"rules","files":["é.py"],
    "selected_rules":["F401","RUF100"], "rule_context": {
        "tables":{"selected":"chosen","disabled":"disabled","external":"external"},
        "configuration":{"external-prefixes":["V101"],"disabled-rules":{"é.py":["F401"]}}
    }});
    if let Some(key) = removed {
        let _removed = value["rule_context"]["configuration"]
            .as_object_mut()
            .expect("fixture object")
            .remove(key);
    }
    if implicit {
        value["rule_context"]["configuration"]["implicit"] = true.into();
    }
    Cow::Owned(value.to_string())
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unicode source path is preserved"),
        word: Word::Succeeds,
        input: || {
            (
                Cow::Borrowed(r#"{"root":"root","rules":"rules","files":["é.txt"],"cache":null}"#),
                Some(unicode_source_path),
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("explicit query module registry is preserved"),
        word: Word::Succeeds,
        input: || {
            (
                Cow::Borrowed(
                    r#"{"root":"root","rules":"rules","query_modules":{"python/ruff/suppress":"fragments/suppress.ryoql"},"files":["source.py"],"cache":null}"#,
                ),
                Some(explicit_query_module_registry),
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown request fields are rejected"),
        word: Word::Refuses,
        input: || {
            (
                Cow::Borrowed(r#"{"root":"root","rules":"rules","files":[],"extra":true}"#),
                None,
            )
        },
        expect: Expect::Error(Refusal::InvalidJson),
    },
    Case {
        name: CaseName("required request fields cannot be omitted"),
        word: Word::Refuses,
        input: || (Cow::Borrowed(r#"{"root":"root","rules":"rules"}"#), None),
        expect: Expect::Error(Refusal::InvalidJson),
    },
    Case {
        name: CaseName("absent cohort keeps the legacy absence"),
        word: Word::Succeeds,
        input: || {
            (
                Cow::Borrowed(
                    r#"{"root":"root","rules":"rules","files":["source.py"],"cache":null}"#,
                ),
                Some(legacy_absence),
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("explicit cohort reaches the worker"),
        word: Word::Succeeds,
        input: || {
            (
                Cow::Borrowed(
                    r#"{"root":"root","rules":"rules","files":["source.py"],"cache":null,"selected_rules":["F401","RUF100"]}"#,
                ),
                Some(explicit_cohort),
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("configured context preserves exact data"),
        word: Word::Succeeds,
        input: || (context_wire(None, false), Some(configured_context)),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("configuration without external prefixes is rejected"),
        word: Word::Refuses,
        input: || (context_wire(Some("external-prefixes"), false), None),
        expect: Expect::Error(Refusal::InvalidJson),
    },
    Case {
        name: CaseName("configuration without disabled rules is rejected"),
        word: Word::Refuses,
        input: || (context_wire(Some("disabled-rules"), false), None),
        expect: Expect::Error(Refusal::InvalidJson),
    },
    Case {
        name: CaseName("configuration with an undeclared key is rejected"),
        word: Word::Refuses,
        input: || (context_wire(None, true), None),
        expect: Expect::Error(Refusal::InvalidJson),
    },
];
fn subject((text, check): Input) -> Result<Value, Failure> {
    WorkerRequest::try_from(WorkerPayload(OsStr::new(&*text)))
        .map(|request| check.is_some_and(|check| check(&request)))
        .map_err(|error| match error {
            ReportsError::Json(_) => Refusal::InvalidJson,
            ReportsError::Invalid(_) | ReportsError::Io(_) | ReportsError::Toml(_) => {
                Refusal::Unexpected
            }
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
