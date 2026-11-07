//! The worker request document carries every explicit input of the invocation.
use crate::outbound::linter::adapters::ryo::types::RequestJson;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, CohortSelection, ContextSeal, QueryModuleName, RuleLabel, WorkerRequest,
};
type Input = (fn() -> WorkerRequest, fn(&serde_json::Value) -> bool);
fn module_registry_request() -> WorkerRequest {
    let mut request = crate::outbound::worker::fixtures::data::request();
    assert!(
        request
            .query_modules
            .insert(
                QueryModuleName("python/ruff/suppress".into()),
                "fragments/suppress.ryoql".into(),
            )
            .is_none()
    );
    let _previous = request.rule_applicability.insert(
        RuleLabel("F401".into()),
        ApplicabilityPolicy("accepted-syntax".into()),
    );
    request
}
fn module_registry_is_transported(value: &serde_json::Value) -> bool {
    value["query_modules"]["python/ruff/suppress"] == "fragments/suppress.ryoql"
        && value["rule_applicability"]["F401"] == "accepted-syntax"
}
fn absent_cohort_request() -> WorkerRequest {
    // The external wire can omit selection; worker preparation still requires resolution.
    let mut request = crate::outbound::worker::fixtures::data::request();
    request.selected_rules = CohortSelection::Frozen;
    request
}
fn legacy_wire_shape(value: &serde_json::Value) -> bool {
    value.get("selected_rules").is_none()
}
fn explicit_cohort_request() -> WorkerRequest {
    let mut request = crate::outbound::worker::fixtures::data::request();
    request.selected_rules = CohortSelection::Explicit(vec![
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
    request.context_manifest = ContextSeal::Sealed(ryo_benchmark_contracts::domain::context::types::ContextManifest {
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
        name: CaseName("request transports the explicit module registry"),
        word: Word::Succeeds,
        input: || (module_registry_request, module_registry_is_transported),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("absent cohort keeps the legacy wire shape"),
        word: Word::Succeeds,
        input: || (absent_cohort_request, legacy_wire_shape),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("explicit cohort is serialized"),
        word: Word::Succeeds,
        input: || (explicit_cohort_request, explicit_cohort_is_serialized),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("request transports the sealed context manifest and its mode"),
        word: Word::Succeeds,
        input: || (listed_context_request, listed_context_is_transported),
        expect: Expect::Value(true),
    },
];
fn subject((build, check): Input) -> Result<Value, Failure> {
    let encoded = RequestJson::try_from(build())
        .map_err(|_rejected_request| ())?
        .0;
    let value: serde_json::Value = serde_json::from_str(&encoded).map_err(|_invalid_json| ())?;
    Ok(check(&value))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
