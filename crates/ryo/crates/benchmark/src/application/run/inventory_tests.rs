//! Two unrelated providers exercise the same inventory behavior and native receipt contract.
use super::*;
use crate::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, ProcessExitCode, ProcessOutput, ResourceObservation,
};
use crate::outbound::inventory::types::{Command, RegisteredRule};
use serde_json::json;
use std::{sync::Mutex, time::Duration};

struct ToyExecution {
    calls: Mutex<Vec<String>>,
}
impl ExecutionPort for ToyExecution {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let name = invocation.arguments[0].to_string_lossy().to_string();
        self.calls.lock().unwrap().push(name.clone());
        if name == "good" {
            let rule = invocation.arguments[1].to_string_lossy();
            let path = invocation
                .environment
                .get(std::ffi::OsStr::new("RYO_VERIFY_CASE_RESULTS"))
                .unwrap();
            let records = ["reference", "candidate"].map(|side| json!({
                "rule": rule, "name": "independent-case", "side": side, "complete": true, "error": null,
                "case_passed": true, "source_sha256": "source-pin", "files": ["input.txt"], "expected": [], "reference_expected": [],
                "diagnostics": [], "skipped": [], "context_manifest": null, "project_context": null,
                "observation_schema": "benchmark-case/v2", "expectation_contract": "diagnostics",
                "execution_metadata": {"availability": "complete"},
                "coverage": {"selected_rule_file_pairs": 1, "evaluated_rule_file_pairs": 1, "skipped_rule_file_pairs": 0, "unknown_or_unread_rule_file_pairs": 0}
            }));
            std::fs::write(
                path,
                records.iter().map(|r| format!("{r}\n")).collect::<String>(),
            )?;
        }
        Ok(ProcessOutput {
            signal: None,
            status: ProcessExitCode(if name == "bad" { 1 } else { 0 }),
            stdout: CapturedBytes(Vec::new()),
            stderr: CapturedBytes(Vec::new()),
            wall: Duration::ZERO,
            resources: ResourceObservation::WallOnly,
        })
    }
}
fn provider(name: &str, middle: &str) -> Registration {
    let defaults = ["missing", middle, "good"]
        .map(|rule| format!("{name}-{rule}"))
        .to_vec();
    let rules = [(middle, defaults[1].clone()), ("good", defaults[2].clone())].into_iter().map(|(mode, rule)| RegisteredRule {
        rule_id: rule.clone(), reason: None,
        command: Some(Command { executable: "existing-native-verifier".into(), arguments: vec![mode.into(), rule.clone()], directory: ".".into(), environment: BTreeMap::new() }),
        cases: vec![json!({"rule": rule, "name": "independent-case", "source_sha256": "source-pin", "files": ["input.txt"], "expected": [], "project_context_mode": "isolated", "reference_discrepancy": null})],
        instance: Some(json!({"input-pin": "immutable-provider-bytes"})),
    }).collect();
    Registration {
        provider: name.into(),
        defaults,
        rules,
        identity: BTreeMap::new(),
    }
}
#[test]
fn independent_providers_keep_missing_and_failed_rules_and_continue() {
    for (name, middle) in [("toy-syntax", "bad"), ("toy-style", "empty")] {
        let root = tempfile::tempdir().unwrap();
        let execution = ToyExecution {
            calls: Mutex::new(Vec::new()),
        };
        assert!(
            !verify(
                &provider(name, middle),
                root.path(),
                &execution,
                &crate::outbound::inventory::factory::local()
            )
            .unwrap()
        );
        let summary: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.path().join("summary.json")).unwrap())
                .unwrap();
        assert_eq!(summary["counts"], json!({"PASS": 1, "FAIL": 2}));
        assert_eq!(summary["rules"].as_array().unwrap().len(), 3);
        assert_eq!(summary["rules"][0]["status"], "FAIL");
        assert_eq!(summary["rules"][2]["status"], "PASS");
        assert!(summary["rules"][2]["case_results_sha256"].is_string());
        assert_eq!(
            *execution.calls.lock().unwrap(),
            [middle.to_string(), "good".into()]
        );
    }
}
#[test]
fn duplicate_and_unknown_registration_never_changes_inventory_cardinality() {
    let root = tempfile::tempdir().unwrap();
    let mut registration = provider("third-provider", "bad");
    registration.rules[0].rule_id = "not-a-pinned-rule".into();
    let execution = ToyExecution {
        calls: Mutex::new(Vec::new()),
    };
    assert!(
        verify(
            &registration,
            root.path(),
            &execution,
            &crate::outbound::inventory::factory::local()
        )
        .is_err()
    );
    let snapshot: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.path().join("summary.json")).unwrap()).unwrap();
    assert_eq!(snapshot["counts"], json!({"PASS": 0, "FAIL": 3}));
    assert!(execution.calls.lock().unwrap().is_empty());
}
