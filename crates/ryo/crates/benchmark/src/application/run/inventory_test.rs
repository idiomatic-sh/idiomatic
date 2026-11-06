//! Two unrelated providers exercise the same inventory behavior and native receipt contract.
use super::*;
use crate::outbound::inventory::types::{
    Command, ExecutablePin, FilePin, RegisteredRule, RegistrationAuthority,
};
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile;
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;
use serde_json::json;
use std::{path::PathBuf, sync::Mutex, time::Duration};

struct ToyExecution {
    calls: Mutex<Vec<String>>,
    worker: PathBuf,
    reference: PathBuf,
}
impl ToyExecution {
    fn for_registration(registration: &Registration) -> Self {
        let RegistrationAuthority::Execution {
            reference, worker, ..
        } = &registration.authority
        else {
            panic!("fixture has execution authority");
        };
        assert_ne!(reference.path, worker.path);
        assert_ne!(reference.sha256, worker.sha256);
        Self {
            calls: Mutex::new(Vec::new()),
            worker: worker.path.clone(),
            reference: reference.path.clone(),
        }
    }
}
impl ExecutionPort for ToyExecution {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        assert_eq!(invocation.executable, self.worker);
        assert_eq!(invocation.arguments[0], "verify-cases");
        assert_eq!(invocation.arguments[1], self.reference.as_os_str());
        assert_eq!(invocation.directory.as_os_str(), invocation.arguments[2]);
        assert!(invocation.environment.is_empty());
        let rule = std::path::Path::new(&invocation.arguments[2])
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let name = rule.rsplit('-').next().unwrap().to_owned();
        self.calls.lock().unwrap().push(name.clone());
        if name == "good" {
            let path = invocation
                .arguments
                .iter()
                .find_map(|argument| argument.to_str()?.strip_prefix("--case-receipts="))
                .map(std::path::PathBuf::from)
                .expect("explicit receipt argument");
            let records = ["reference", "candidate"].map(|side| json!({
                "rule": rule, "name": "independent-case", "side": side, "complete": true, "error": null, "documented_reference_discrepancy": false,
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
fn provider(name: &str, middle: &str) -> (Registration, tempfile::TempDir) {
    use sha2::Digest as _;
    let context = tempfile::tempdir().unwrap();
    let reference = context.path().join("native-reference");
    let worker = context.path().join("native-worker");
    std::fs::write(&reference, b"pinned reference executable").unwrap();
    std::fs::write(&worker, b"distinct pinned worker executable").unwrap();
    let digest = |path: &std::path::Path| {
        format!("{:x}", sha2::Sha256::digest(std::fs::read(path).unwrap()))
    };
    let reference_sha = digest(&reference);
    let worker_sha = digest(&worker);
    assert_ne!(reference_sha, worker_sha);
    let defaults = ["missing", middle, "good"]
        .map(|rule| format!("{name}-{rule}"))
        .to_vec();
    let profile = context.path().join("benchmark.toml");
    std::fs::write(&profile, format!("tool = \"fixture\"\nversion = \"1\"\nsha256 = [{reference_sha:?}]\ndefaults = {defaults:?}\n")).unwrap();
    let rules = [(middle, defaults[1].clone()), ("good", defaults[2].clone())]
        .into_iter()
        .map(|(mode, rule)| {
            let directory = context.path().join(&rule);
            std::fs::create_dir_all(directory.join("evidence")).unwrap();
            let definition = directory.join("query.ryoql");
            std::fs::write(&definition, format!("definition for {rule}")).unwrap();
            let evidence = directory.join("evidence/oracle.json");
            std::fs::write(&evidence, format!("independent evidence for {rule}")).unwrap();
            let context_profile = directory.join("benchmark.toml");
            std::fs::write(
                &context_profile,
                format!(
                    "tool='fixture'\nversion='1'\nsha256=['{reference_sha}']\ndefaults=['{rule}']\n"
                ),
            )
            .unwrap();
            let pin = FilePin {
                sha256: digest(&definition),
                path: definition.clone(),
            };
            RegisteredRule {
                definition: Some(pin),
                modules: BTreeMap::new(),
                artifacts: BTreeMap::from([
                    (definition.clone(), digest(&definition)),
                    (evidence.clone(), digest(&evidence)),
                    (context_profile.clone(), digest(&context_profile)),
                ]),
                rule_id: rule.clone(),
                reason: None,
                command: Some(Command::CaseVerification {
                    context: directory.clone(),
                }),
                cases: vec![
                    crate::application::inventory_receipts::fixtures::registered_case(
                        &rule,
                        "independent-case",
                    ),
                ],
                instance: Some(json!({"input-pin": "immutable-provider-bytes"})),
            }
        })
        .collect();
    (
        Registration {
            authority: RegistrationAuthority::Execution {
                profile: context.path().to_owned(),
                profile_sha256: digest(&profile),
                reference: ExecutablePin {
                    path: reference,
                    sha256: reference_sha,
                },
                worker: ExecutablePin {
                    path: worker,
                    sha256: worker_sha,
                },
            },
            provider: name.into(),
            defaults,
            rules,
            identity: BTreeMap::new(),
        },
        context,
    )
}
#[test]
fn independent_providers_keep_missing_and_failed_rules_and_continue() {
    for (name, middle) in [("toy-syntax", "bad"), ("toy-style", "empty")] {
        let root = tempfile::tempdir().unwrap();
        let (registration, _context) = provider(name, middle);
        let execution = ToyExecution::for_registration(&registration);
        assert!(
            !verify(
                &registration,
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
    let (mut registration, _context) = provider("third-provider", "bad");
    registration.rules[0].rule_id = "not-a-pinned-rule".into();
    let execution = ToyExecution::for_registration(&registration);
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

#[test]
fn inventory_rejects_undeclared_module_pins_before_each_native_invocation() {
    use sha2::Digest as _;
    let (mut registration, _context) = provider("module-provider", "bad");
    for entry in &mut registration.rules {
        let Command::CaseVerification { context } = entry.command.as_ref().unwrap();
        let helper = context.join("helper.ryoql");
        std::fs::write(&helper, "declared helper").unwrap();
        let profile = context.join("benchmark.toml");
        let content = std::fs::read_to_string(&profile).unwrap()
            + "query-modules={'test/helper'='helper.ryoql'}\n";
        std::fs::write(&profile, content).unwrap();
        for path in [profile, helper] {
            let digest = format!("{:x}", sha2::Sha256::digest(std::fs::read(&path).unwrap()));
            entry.artifacts.insert(path, digest);
        }
        assert!(entry.modules.is_empty()); // The omission under test; all actual bytes stay pinned.
    }
    let output = tempfile::tempdir().unwrap();
    let execution = ToyExecution::for_registration(&registration);
    assert!(
        !verify(
            &registration,
            output.path(),
            &execution,
            &crate::outbound::inventory::factory::local()
        )
        .unwrap()
    );
    assert!(execution.calls.lock().unwrap().is_empty());
    let summary: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.path().join("summary.json")).unwrap())
            .unwrap();
    assert_eq!(summary["counts"], json!({"PASS":0,"FAIL":3}));
    for row in &summary["rules"].as_array().unwrap()[1..] {
        assert!(
            row["reason"]
                .as_str()
                .unwrap()
                .contains("registered module inventory differs from pinned context profile")
        );
    }
}
