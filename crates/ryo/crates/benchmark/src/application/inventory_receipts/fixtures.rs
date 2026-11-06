//! Registered cases and native observation rows the receipt validation is proven over.
use ryo_benchmark_contracts::domain::artifacts::types::{
    FileExecutable, FileMetadataEntry, digest::value::ArtifactDigest,
};
use serde_json::{Value, json};

/// The registered documents and the native rows observed for them.
pub type Documents = (Vec<Value>, Vec<Value>);

fn remove(document: &mut Value, key: &str) {
    if let Some(object) = document.as_object_mut() {
        let _removed = object.remove(key);
    }
}

fn coverage() -> Value {
    json!({"selected_rule_file_pairs": 1, "evaluated_rule_file_pairs": 1,
           "skipped_rule_file_pairs": 0, "unknown_or_unread_rule_file_pairs": 0})
}

fn registered(name: &str) -> Value {
    json!({
        "rule": "A001", "name": name, "source_sha256": "source-pin",
        "files": ["input.txt"], "expected": [],
        "project_context_mode": "isolated", "reference_discrepancy": null
    })
}

fn observed(name: &str) -> Value {
    json!({
        "rule": "A001", "name": name, "complete": true, "error": null,
        "case_passed": true, "source_sha256": "source-pin", "files": ["input.txt"],
        "expected": [], "reference_expected": [], "diagnostics": [], "skipped": [],
        "context_manifest": null, "project_context": null,
        "observation_schema": "benchmark-case/v2", "expectation_contract": "diagnostics",
        "execution_metadata": {"availability": "complete"}, "coverage": coverage()
    })
}

/// Both native sides of one named case.
fn sides(base: &Value) -> Vec<Value> {
    ["reference", "candidate"]
        .into_iter()
        .map(|side| {
            let mut row = base.clone();
            row["side"] = json!(side);
            row
        })
        .collect()
}

/// One registered case with both complete, passing native sides.
pub fn passing() -> Documents {
    (vec![registered("case")], sides(&observed("case")))
}

/// The same registered case, observed with an extra diagnostic on the candidate side.
pub fn candidate_with_unexpected_diagnostic() -> Documents {
    let (cases, mut rows) = passing();
    rows[1]["diagnostics"] = json!([{"rule": "A001", "file": "input.txt", "start": 0, "end": 1}]);
    (cases, rows)
}

/// The candidate diagnostics are an object instead of an array.
pub fn candidate_diagnostics_not_an_array() -> Documents {
    let (cases, mut rows) = passing();
    rows[1]["diagnostics"] = json!({"not": "an array"});
    (cases, rows)
}

/// Two registered cases and the rows of one case and an unrelated one.
fn first_and_unrelated() -> Vec<Value> {
    let mut rows = sides(&observed("first"));
    rows.extend(sides(&observed("unrelated")));
    rows
}

/// The same case registered twice.
pub fn duplicate_registration() -> Documents {
    (
        vec![registered("first"), registered("first")],
        first_and_unrelated(),
    )
}

/// Registered identities that do not match the observed rows.
pub fn unregistered_observation() -> Documents {
    (
        vec![registered("second"), registered("first")],
        first_and_unrelated(),
    )
}

/// Every row observed twice.
pub fn duplicate_observation() -> Documents {
    let rows = sides(&observed("first"));
    let mut doubled = rows.clone();
    doubled.extend(rows);
    (vec![registered("second"), registered("first")], doubled)
}

fn discrepancy_finding() -> Value {
    json!({"rule": "A001", "file": "input.txt", "start": 0, "end": 1})
}

fn documented_discrepancy(settings: Value) -> Documents {
    let finding = discrepancy_finding();
    let mut case = registered("case");
    case["reference_discrepancy"] = json!({
        "intended_expected": [], "reference_expected": [finding],
        "approved_executable_sha256": ["tool-pin"],
        "identity": {
            "settings-sha256": settings, "profile-sha256": "profile-pin",
            "source-sha256": "source-pin", "reference-output-sha256": "reference-pin",
            "intended-output-sha256": "intended-pin"
        }
    });
    let proof = json!({
        "actual_executable_sha256": "tool-pin", "settings_sha256": settings,
        "profile_sha256": "profile-pin", "source_sha256": "source-pin",
        "reference_output_sha256": "reference-pin", "intended_output_sha256": "intended-pin"
    });
    let mut base = observed("case");
    base["reference_expected"] = json!([finding]);
    base["discrepancy_validation"] = proof;
    base["documented_reference_discrepancy"] = json!(true);
    base["documented_tool_contract_difference"] = json!(false);
    let mut rows = sides(&base);
    rows[0]["diagnostics"] = json!([finding]);
    (vec![case], rows)
}

/// A reference difference bound to independent evidence on both sides.
pub fn independently_bound_discrepancy() -> Documents {
    documented_discrepancy(json!("settings-pin"))
}

/// The same difference whose settings identity is unbound.
pub fn unbound_discrepancy() -> Documents {
    documented_discrepancy(Value::Null)
}

/// One file whose executable bit is declared and observed on both sides.
pub fn with_file_metadata(executable: bool) -> Documents {
    let flag = if executable {
        FileExecutable::Set
    } else {
        FileExecutable::Unset
    };
    let values =
        std::collections::BTreeMap::from([(ryo::linter::RelativePath("input.txt".into()), flag)]);
    let digest = ArtifactDigest::file_metadata(Some(values.iter().map(|(path, executable)| {
        FileMetadataEntry {
            path,
            executable: *executable,
        }
    })))
    .ok()
    .flatten()
    .map(|value| value.0);
    let mut case = registered("mode");
    case["file_observation"] = json!("posix-executable");
    case["file_metadata"] = json!({"input.txt": executable});
    case["file_metadata_sha256"] = json!(digest);
    let mut base = observed("mode");
    base["file_metadata_sha256"] = json!(digest);
    (vec![case], sides(&base))
}

/// The declaration of the other mode, kept against rows observed for this one.
pub fn declaring_the_other_mode() -> Documents {
    (with_file_metadata(true).0, with_file_metadata(false).1)
}

/// The registered case lost one of its metadata keys.
pub fn without_case_key(key: &'static str) -> Documents {
    let (mut cases, rows) = with_file_metadata(false);
    remove(&mut cases[0], key);
    (cases, rows)
}

/// Both native rows lost their metadata identity.
pub fn rows_without_metadata_identity() -> Documents {
    let (cases, mut rows) = with_file_metadata(false);
    for row in &mut rows {
        remove(row, "file_metadata_sha256");
    }
    (cases, rows)
}

/// A case that never declared file metadata at all.
pub fn unspecified_legacy() -> Documents {
    let (mut cases, mut rows) = with_file_metadata(false);
    for key in ["file_observation", "file_metadata", "file_metadata_sha256"] {
        remove(&mut cases[0], key);
    }
    for row in &mut rows {
        remove(row, "file_metadata_sha256");
    }
    (cases, rows)
}

/// The project scope and inventory are declared and their identity bound on all three documents.
pub fn project_scope(roots: Value, state: &str, row_identity: Option<&str>) -> Documents {
    let mut declared = with_file_metadata(false);
    declared.0[0]["project_search_roots"] = json!(["src"]);
    declared.0[0]["given_project_inventory"] = json!({
        "completeness": "partial", "entries": [{"path": "src/localmod.py", "state": "file"}]
    });
    let digest = crate::domain::registered_inputs::service::project_inputs_digest(&declared.0[0])
        .unwrap_or(Value::Null);
    declared.0[0]["project_inputs_sha256"] = digest.clone();
    for row in &mut declared.1 {
        row["project_inputs_sha256"] = digest.clone();
    }
    declared.0[0]["project_search_roots"] = roots;
    declared.0[0]["given_project_inventory"]["entries"][0]["state"] = json!(state);
    if let Some(identity) = row_identity {
        declared.1[0]["project_inputs_sha256"] = json!(identity);
    }
    declared
}

/// An explicit cohort declared on the case and on both rows.
pub fn with_selected_rules(
    case_cohort: Option<Value>,
    row_cohorts: [Option<Value>; 2],
) -> Documents {
    let (mut cases, mut rows) = with_file_metadata(false);
    for (target, cohort) in
        std::iter::once((&mut cases[0], case_cohort)).chain(rows.iter_mut().zip(row_cohorts))
    {
        match cohort {
            Some(value) => target["selected_rules"] = value,
            None => remove(target, "selected_rules"),
        }
    }
    (cases, rows)
}

/// The observation declarations declared on the case and on both rows.
pub fn with_rule_observations(
    case_value: Option<Value>,
    row_values: [Option<Value>; 2],
) -> Documents {
    let (mut cases, mut rows) = with_file_metadata(false);
    for (target, value) in
        std::iter::once((&mut cases[0], case_value)).chain(rows.iter_mut().zip(row_values))
    {
        match value {
            Some(value) => target["rule_observations"] = value,
            None => remove(target, "rule_observations"),
        }
    }
    (cases, rows)
}

/// The declaration both sides must carry.
pub fn observation_declaration(output: &str) -> Value {
    json!({"A001": [{"name": "matched", "columns": [{"name": "file", "kind": "file"}],
        "producers": [{"kind": "output", "rule": "B002", "output": output}]}]})
}

/// The documents `base` builds, with one edit applied.
pub fn edited(base: fn() -> Documents, edit: fn(&mut Documents)) -> Documents {
    let mut documents = base();
    edit(&mut documents);
    documents
}
