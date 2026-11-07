//! Two unrelated providers exercise the same inventory behavior and native receipt contract.
use super::constants;
use super::service::verify;
use crate::domain::suite::types::inventory::value::InventoryVerdict;
use crate::domain::discrepancy::types::Contestant;
use crate::outbound::{
    execution::interface::ExecutionPort,
    inventory::types::{
        ProviderName, RegisteredModuleName, Registration, RuleCommand, RuleDefinition,
        RuleInstance, RuleReason,
    },
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{CanonicalJson, RuleName, Sha256Text};
use ryo_benchmark_contracts::domain::receipts::constants::{CASE_RECEIPTS_ARGUMENT, VERIFY_CASES};
use ryo_benchmark_contracts::domain::measurements::types::{Invocation, TerminationSignal};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::types::{Command, ExecutablePin, FilePin, RegisteredRule, RegistrationAuthority};
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile;
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;
use serde_json::json;
use std::{borrow::Cow, path::PathBuf, sync::Mutex, time::Duration};

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
        assert_eq!(invocation.arguments[0], VERIFY_CASES);
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
                .find_map(|argument| argument.to_str()?.strip_prefix(CASE_RECEIPTS_ARGUMENT))
                .map(std::path::PathBuf::from)
                .expect("explicit receipt argument");
            let records = [Contestant::Reference, Contestant::Candidate].map(|side| json!({
                "rule": rule, "name": "independent-case", "side": side.to_string(), "complete": true, "error": null, "documented_reference_discrepancy": false,
                "case_passed": true, "source_sha256": "source-pin", "files": ["input.txt"], "expected": [], "reference_expected": [],
                "diagnostics": [], "skipped": [], "context_manifest": null, "project_context": null,
                "observation_schema": crate::domain::suite::constants::BENCHMARK_CASE_V2, "expectation_contract": "diagnostics",
                "execution_metadata": {"availability": "complete"},
                "coverage": {"selected_rule_file_pairs": 1, "evaluated_rule_file_pairs": 1, "skipped_rule_file_pairs": 0, "unknown_or_unread_rule_file_pairs": 0}
            }));
            std::fs::write(
                path,
                records.iter().map(|r| format!("{r}\n")).collect::<String>(),
            )?;
        }
        Ok(ProcessOutput {
            signal: TerminationSignal::Exited,
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
    let profile = context.path().join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST);
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
            let context_profile = directory.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST);
            std::fs::write(
                &context_profile,
                format!(
                    "tool='fixture'\nversion='1'\nsha256=['{reference_sha}']\ndefaults=['{rule}']\n"
                ),
            )
            .unwrap();
            let pin = FilePin {
                sha256: Sha256Text(digest(&definition)),
                path: definition.clone(),
            };
            RegisteredRule {
                definition: RuleDefinition::Pinned(pin),
                modules: BTreeMap::<RegisteredModuleName, FilePin>::new(),
                artifacts: BTreeMap::from([
                    (definition.clone(), Sha256Text(digest(&definition))),
                    (evidence.clone(), Sha256Text(digest(&evidence))),
                    (context_profile.clone(), Sha256Text(digest(&context_profile))),
                ]),
                rule_id: RuleName(rule.clone()),
                reason: RuleReason::Unstated,
                command: RuleCommand::Registered(Command::CaseVerification {
                    context: directory.clone(),
                }),
                cases: vec![
                    crate::domain::inventory_receipts::fixtures::registered_case(
                        &rule,
                        "independent-case",
                    ),
                ],
                instance: RuleInstance::Declared(CanonicalJson(
                    json!({"input-pin": "immutable-provider-bytes"}).to_string(),
                )),
            }
        })
        .collect();
    (
        Registration {
            authority: RegistrationAuthority::Execution {
                profile: context.path().to_owned(),
                profile_sha256: Sha256Text(digest(&profile)),
                reference: ExecutablePin {
                    path: reference,
                    sha256: Sha256Text(reference_sha),
                },
                worker: ExecutablePin {
                    path: worker,
                    sha256: Sha256Text(worker_sha),
                },
            },
            provider: ProviderName(name.into()),
            defaults: defaults.into_iter().map(RuleName).collect(),
            rules,
            identity: BTreeMap::new(),
        },
        context,
    )
}
/// The registrations the inventory is verified over.
#[derive(Clone, Copy)]
enum Scenario {
    /// Two unrelated providers whose middle rules fail or are empty.
    IndependentProviders,
    /// A registration whose first rule is not a pinned default.
    UnknownRule,
    /// Rules whose profile declares a module the registration never pinned.
    UndeclaredModules,
}

type Input = Scenario;
type Value = InventoryVerdict;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("independent providers keep missing and failed rules and continue"),
        word: Word::Succeeds,
        input: || Scenario::IndependentProviders,
        expect: Expect::Value(InventoryVerdict::Failed),
    },
    Case {
        name: CaseName("undeclared module pins fail every rule before each native invocation"),
        word: Word::Succeeds,
        input: || Scenario::UndeclaredModules,
        expect: Expect::Value(InventoryVerdict::Failed),
    },
    Case {
        name: CaseName("an unknown registered rule never changes the inventory cardinality"),
        word: Word::Refuses,
        input: || Scenario::UnknownRule,
        expect: Expect::Error(Cow::Borrowed(
            constants::UNKNOWN_OR_DUPLICATE_REGISTERED_RULE,
        )),
    },
];

fn summary_of(root: &std::path::Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(root.join("summary.json")).unwrap()).unwrap()
}

fn independent_providers() -> Result<InventoryVerdict, Error> {
    let mut verified = InventoryVerdict::Failed;
    for (name, middle) in [("toy-syntax", "bad"), ("toy-style", "empty")] {
        let root = tempfile::tempdir().unwrap();
        let (registration, _context) = provider(name, middle);
        let execution = ToyExecution::for_registration(&registration);
        verified = verify(
            &registration,
            root.path(),
            &crate::outbound::suite::factory::local(),
            &crate::outbound::receipts::factory::local(
                crate::outbound::receipts::types::CaseReceipts::Unrecorded,
            ),
            &execution,
            &crate::outbound::inventory::factory::local(),
        )?;
        let summary = summary_of(root.path());
        assert_eq!(summary["counts"], json!({"PASS": 1, "FAIL": 2}));
        assert_eq!(summary["rules"].as_array().unwrap().len(), 3);
        assert_eq!(summary["rules"][0]["status"], "FAIL");
        assert_eq!(summary["rules"][2]["status"], "PASS");
        assert!(summary["rules"][2]["case_results_sha256"].is_string());
        assert_eq!(
            *execution.calls.lock().unwrap(),
            [middle.to_string(), "good".into()]
        );
        if verified == InventoryVerdict::Passed {
            return Ok(verified);
        }
    }
    Ok(verified)
}

fn unknown_rule() -> Result<InventoryVerdict, Error> {
    let root = tempfile::tempdir().unwrap();
    let (mut registration, _context) = provider("third-provider", "bad");
    registration.rules[0].rule_id = RuleName("not-a-pinned-rule".into());
    let execution = ToyExecution::for_registration(&registration);
    let answer = verify(
        &registration,
        root.path(),
        &crate::outbound::suite::factory::local(),
        &crate::outbound::receipts::factory::local(
            crate::outbound::receipts::types::CaseReceipts::Unrecorded,
        ),
        &execution,
        &crate::outbound::inventory::factory::local(),
    );
    let snapshot = summary_of(root.path());
    assert_eq!(snapshot["counts"], json!({"PASS": 0, "FAIL": 3}));
    assert!(execution.calls.lock().unwrap().is_empty());
    answer
}

fn undeclared_modules() -> Result<InventoryVerdict, Error> {
    use sha2::Digest as _;
    let (mut registration, _context) = provider("module-provider", "bad");
    for entry in &mut registration.rules {
        let RuleCommand::Registered(Command::CaseVerification { context }) = &entry.command else {
            panic!("fixture rules are registered");
        };
        let helper = context.join("helper.ryoql");
        std::fs::write(&helper, "declared helper").unwrap();
        let profile = context.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST);
        let content = std::fs::read_to_string(&profile).unwrap()
            + "query-modules={'test/helper'='helper.ryoql'}\n";
        std::fs::write(&profile, content).unwrap();
        for path in [profile, helper] {
            let digest = format!("{:x}", sha2::Sha256::digest(std::fs::read(&path).unwrap()));
            entry.artifacts.insert(path, Sha256Text(digest));
        }
        assert!(entry.modules.is_empty()); // The omission under test; all actual bytes stay pinned.
    }
    let output = tempfile::tempdir().unwrap();
    let execution = ToyExecution::for_registration(&registration);
    let answer = verify(
        &registration,
        output.path(),
        &crate::outbound::suite::factory::local(),
        &crate::outbound::receipts::factory::local(
            crate::outbound::receipts::types::CaseReceipts::Unrecorded,
        ),
        &execution,
        &crate::outbound::inventory::factory::local(),
    );
    assert!(execution.calls.lock().unwrap().is_empty());
    let summary = summary_of(output.path());
    assert_eq!(summary["counts"], json!({"PASS":0,"FAIL":3}));
    for row in &summary["rules"].as_array().unwrap()[1..] {
        assert!(
            row["reason"]
                .as_str()
                .unwrap()
                .contains(constants::REGISTERED_MODULE_INVENTORY_DIFFERS_FROM_PIN)
        );
    }
    answer
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::IndependentProviders => independent_providers(),
        Scenario::UnknownRule => unknown_rule(),
        Scenario::UndeclaredModules => undeclared_modules(),
    }
    .map_err(|error| error.to_string().into())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
