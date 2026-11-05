//! Registered certification retains independent artifact namespaces and refuses changed authority.
use super::{interface::CertifyUseCase as _, use_case::CertifyUseCaseImpl};
use crate::outbound::execution::interface::ExecutionPort;
use ryo_benchmark_contracts::domain::measurements::types::*;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;
use ryo_benchmark_contracts::domain::requests::types::CertificationCases;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

struct Native {
    launches: Mutex<usize>,
    worker: PathBuf,
    reference: PathBuf,
}
impl ExecutionPort for Native {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn run(&self, request: Invocation) -> Result<ProcessOutput, ReportsError> {
        *self.launches.lock().unwrap() += 1;
        assert_eq!(request.executable, self.worker);
        assert_eq!(request.arguments[1], self.reference.as_os_str());
        assert_eq!(request.directory.as_os_str(), request.arguments[2]);
        assert!(request.environment.is_empty());
        assert_eq!(request.arguments[0], "verify-cases");
        let rule = Path::new(&request.arguments[2])
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        let receipt = request
            .arguments
            .iter()
            .find_map(|a| a.to_str()?.strip_prefix("--case-receipts="))
            .unwrap();
        let records = ["reference", "candidate"].map(|side| json!({
            "rule":rule,"name":"independent","side":side,"complete":true,"error":null,
            "case_passed":true,"source_sha256":"source","files":["input.txt"],"expected":[],
            "reference_expected":[],"diagnostics":[],"skipped":[],"context_manifest":null,"project_context":null,
            "observation_schema":"benchmark-case/v2","expectation_contract":"diagnostics",
            "execution_metadata":{"availability":"complete"},
            "coverage":{"selected_rule_file_pairs":1,"evaluated_rule_file_pairs":1,"skipped_rule_file_pairs":0,"unknown_or_unread_rule_file_pairs":0}
        }));
        std::fs::write(
            receipt,
            records.iter().map(|r| format!("{r}\n")).collect::<String>(),
        )?;
        Ok(ProcessOutput {
            status: ProcessExitCode(0),
            signal: None,
            stdout: CapturedBytes(vec![]),
            stderr: CapturedBytes(vec![]),
            wall: Duration::ZERO,
            resources: ResourceObservation::WallOnly,
        })
    }
}
fn digest(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}
fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}
struct Fixture {
    _root: tempfile::TempDir,
    profile: PathBuf,
    reference_executable: PathBuf,
    worker_executable: PathBuf,
    registration: PathBuf,
    document: Value,
    execution: Arc<Native>,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let profile = root.path().join("measured");
        let reference_executable = root.path().join(super::constants::FIXTURE_TOOL_FILE);
        let worker_executable = root.path().join("independent-worker");
        write(&reference_executable, super::constants::FIXTURE_TOOL_SCRIPT);
        write(
            &worker_executable,
            "#!/bin/sh\nprintf 'independent worker\\n'\n",
        );
        assert_ne!(digest(&reference_executable), digest(&worker_executable));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            for executable in [&reference_executable, &worker_executable] {
                std::fs::set_permissions(executable, std::fs::Permissions::from_mode(0o755))
                    .unwrap();
            }
        }
        write(&profile.join("helper.ryoql"), "shared tested helper");
        write(&profile.join("second.ryoql"), "second tested helper");
        write(
            &profile.join("benchmark.toml"),
            &format!(
                "tool='fixture'\nversion='1'\nsha256=['{}']\ndefaults=['A','B']\nquery-modules={{'test/helper'='helper.ryoql','test/second'='second.ryoql'}}\n",
                digest(&reference_executable)
            ),
        );
        let entries = ["A","B"].map(|rule| {
            write(&profile.join(format!("rules/{rule}/query.ryoql")), &format!("query {rule}"));
            write(&profile.join(format!("rules/{rule}/cases.toml")), "[[cases]]\nname='positive'\nfiles={'input.txt'='x'}\nexpected=[{file='input.txt',start=0,end=1}]\n[[cases]]\nname='negative'\nfiles={'input.txt'='y'}\nexpected=[]\n");
            let context = root.path().join(format!("registered/{rule}"));
            let definition = context.join(format!("rules/{rule}/query.ryoql"));
            let helper = context.join("helper.ryoql");
            let second = context.join("second.ryoql");
            write(&second,"second tested helper");
            let evidence = context.join("evidence/oracle.json");
            write(&definition,&format!("query {rule}")); write(&helper,"shared tested helper"); write(&evidence,&format!("independent {rule} evidence"));
            write(&context.join("benchmark.toml"), &format!("tool='fixture'\nversion='1'\nsha256=['{}']\ndefaults=['{rule}']\nquery-modules={{'test/helper'='helper.ryoql','test/second'='second.ryoql'}}\n",digest(&reference_executable)));
            let artifacts = [definition.clone(),helper.clone(),second.clone(),evidence,context.join("benchmark.toml")].into_iter().map(|p|(p.display().to_string(),json!(digest(&p)))).collect::<serde_json::Map<_,_>>();
            json!({"rule_id":rule,"reason":null,"instance":null,"definition":{"path":definition,"sha256":digest(&definition)},"modules":{"test/helper":{"path":helper,"sha256":digest(&helper)},"test/second":{"path":second,"sha256":digest(&second)}},"artifacts":artifacts,
                "command":{"kind":"case-verification","context":context},
                "cases":[{"rule":rule,"name":"independent","source_sha256":"source","files":["input.txt"],"expected":[],"project_context_mode":"isolated","reference_discrepancy":null}]})
        });
        let document = json!({"authority":{"kind":"execution","profile":profile,"profile_sha256":digest(&profile.join("benchmark.toml")),"reference":{"path":reference_executable,"sha256":digest(&reference_executable)},"worker":{"path":worker_executable,"sha256":digest(&worker_executable)}},"provider":"neutral","defaults":["A","B"],"rules":entries,"identity":{}});
        let registration = root.path().join("registration.json");
        std::fs::write(&registration, serde_json::to_vec(&document).unwrap()).unwrap();
        Self {
            _root: root,
            profile,
            reference_executable: reference_executable.clone(),
            worker_executable: worker_executable.clone(),
            registration,
            document,
            execution: Arc::new(Native {
                launches: Mutex::new(0),
                worker: worker_executable,
                reference: reference_executable,
            }),
        }
    }
    fn certify(
        &self,
    ) -> Result<crate::domain::suite::types::certificate::value::Certificate, ReportsError> {
        self.certify_profile(&self.profile)
    }
    fn certify_profile(
        &self,
        measured_profile: &Path,
    ) -> Result<crate::domain::suite::types::certificate::value::Certificate, ReportsError> {
        self.certify_profile_rules(measured_profile, &self.profile.join("rules"))
    }
    fn certify_profile_rules(
        &self,
        measured_profile: &Path,
        selected_rules: &Path,
    ) -> Result<crate::domain::suite::types::certificate::value::Certificate, ReportsError> {
        let operation = CertifyUseCaseImpl::new(
            Arc::new(crate::outbound::suite::factory::local()),
            Arc::new(crate::outbound::linter::fixtures::fake::LinterFake::default()),
            Arc::new(crate::outbound::linter::fixtures::fake::LinterFake::default()),
            self.execution.clone(),
            Arc::new(crate::outbound::inventory::factory::local()),
        );
        operation.certify(
            measured_profile,
            &self.profile,
            &self.reference_executable,
            &self.worker_executable,
            selected_rules,
            &self._root.path().join("scratch"),
            BenchmarkCoverage::Accounted,
            &CertificationCases::Registered {
                registration: self.registration.clone(),
                output: self._root.path().join("receipts"),
            },
        )
    }
    fn save(&self) {
        std::fs::write(
            &self.registration,
            serde_json::to_vec(&self.document).unwrap(),
        )
        .unwrap();
    }
}
fn copy_measured_profile(fixture: &Fixture, destination: &Path) {
    for relative in [
        "benchmark.toml",
        "helper.ryoql",
        "second.ryoql",
        "rules/A/query.ryoql",
        "rules/A/cases.toml",
        "rules/B/query.ryoql",
        "rules/B/cases.toml",
    ] {
        write(
            &destination.join(relative),
            &std::fs::read_to_string(fixture.profile.join(relative)).unwrap(),
        );
    }
}
#[test]
fn registered_contexts_execute_and_keep_colliding_relative_evidence_names_separate() {
    let f = Fixture::new();
    assert_ne!(f.reference_executable, f.worker_executable);
    assert_ne!(
        digest(&f.reference_executable),
        digest(&f.worker_executable)
    );
    let certificate = f.certify().unwrap();
    assert_eq!(*f.execution.launches.lock().unwrap(), 2);
    assert!(matches!(
        certificate.case_contexts(),
        CertificationCases::Registered { .. }
    ));
    for rule in ["A", "B"] {
        assert!(
            f._root
                .path()
                .join(format!("receipts/profiles/{rule}/cases.jsonl"))
                .is_file()
        );
    }
}
#[test]
fn swapping_distinct_measured_contestants_refuses_before_native_launch() {
    use crate::outbound::suite::interface::SuitePort as _;
    let f = Fixture::new();
    let suite = crate::outbound::suite::factory::local();
    let tool = suite
        .profile(crate::outbound::suite::types::ArtifactPath(&f.profile))
        .unwrap();
    let error = super::case_inventory::prepare(
        &crate::outbound::inventory::factory::local(),
        &suite,
        &f.registration,
        &f.profile,
        &tool,
        &f.profile.join("rules"),
        &f.worker_executable,
        &f.reference_executable,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("case inventory executable identities differ from measured contestants"),
        "{error}"
    );
    assert_eq!(*f.execution.launches.lock().unwrap(), 0);
}
#[test]
fn registered_context_drift_and_untrusted_authority_refuse_before_native_launch() {
    for change in ["artifact", "query", "module", "receipt-only"] {
        let mut f = Fixture::new();
        match change {
            "artifact" => write(
                Path::new(
                    f.document["rules"][0]["artifacts"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .find(|p| p.ends_with("oracle.json"))
                        .unwrap(),
                ),
                "changed",
            ),
            "query" => write(
                &f.profile.join("rules/A/query.ryoql"),
                "different measured query",
            ),
            "module" => write(&f.profile.join("helper.ryoql"), "different measured helper"),
            "receipt-only" => {
                f.document["authority"] = json!({"kind":"receipt-only"});
                f.save();
            }
            _ => unreachable!(),
        }
        let error = f.certify().unwrap_err().to_string();
        let expected = match change {
            "artifact" => "registered case artifact changed",
            "query" => "measured query differs",
            "module" => "measured helper differs",
            "receipt-only" => "receipt-only inventory cannot authorize execution",
            _ => unreachable!(),
        };
        assert!(error.contains(expected), "{change}: {error}");
        assert_eq!(*f.execution.launches.lock().unwrap(), 0, "{change}");
    }
}
#[test]
fn missing_registered_rule_never_issues_a_certificate() {
    let mut f = Fixture::new();
    f.document["rules"].as_array_mut().unwrap().pop();
    f.save();
    assert!(f.certify().is_err());
    assert_eq!(*f.execution.launches.lock().unwrap(), 1);
}

#[test]
fn invocation_and_canonical_profile_substitution_refuse_before_launch() {
    for change in [
        "executable",
        "arguments",
        "directory",
        "environment",
        "context",
        "extra-context-file",
        "foreign-profile",
    ] {
        let mut f = Fixture::new();
        match change {
            "executable" => {
                f.document["rules"][0]["command"]["executable"] = json!("foreign-worker")
            }
            "arguments" => {
                f.document["rules"][0]["command"]["arguments"] = json!(["different-operation"])
            }
            "directory" => {
                f.document["rules"][0]["command"]["directory"] = json!("foreign-context")
            }
            "environment" => {
                f.document["rules"][0]["command"]["environment"] = json!({"CONFIG":"foreign"})
            }
            "context" => {
                f.document["rules"][0]["command"]["context"] =
                    f.document["rules"][1]["command"]["context"].clone()
            }
            "extra-context-file" => write(
                &PathBuf::from(
                    f.document["rules"][0]["command"]["context"]
                        .as_str()
                        .unwrap(),
                )
                .join("rogue-config.toml"),
                "changed configuration",
            ),
            "foreign-profile" => {
                let foreign = f._root.path().join("foreign");
                let bytes = std::fs::read_to_string(f.profile.join("benchmark.toml")).unwrap();
                write(
                    &foreign.join("benchmark.toml"),
                    &(bytes + "parser-configuration='different-parser'\n"),
                );
                f.document["authority"]["profile"] = json!(foreign);
                f.document["authority"]["profile_sha256"] =
                    json!(digest(&foreign.join("benchmark.toml")));
            }
            _ => unreachable!(),
        }
        f.save();
        let error = f.certify().unwrap_err().to_string();
        let expected = match change {
            "context" | "extra-context-file" => "registered context namespace differs",
            "foreign-profile" => "registered authority differs from measured canonical profile",
            _ => "unknown field",
        };
        assert!(error.contains(expected), "{change}: {error}");
        assert_eq!(*f.execution.launches.lock().unwrap(), 0, "{change}");
    }
}

#[test]
fn measured_profile_identity_cannot_be_substituted_via_the_query_directory() {
    let f = Fixture::new();
    let foreign = f._root.path().join("measured-foreign");
    for relative in [
        "helper.ryoql",
        "second.ryoql",
        "rules/A/query.ryoql",
        "rules/A/cases.toml",
        "rules/B/query.ryoql",
        "rules/B/cases.toml",
    ] {
        write(
            &foreign.join(relative),
            &std::fs::read_to_string(f.profile.join(relative)).unwrap(),
        );
    }
    let bytes = std::fs::read_to_string(f.profile.join("benchmark.toml")).unwrap();
    write(
        &foreign.join("benchmark.toml"),
        &(bytes + "parser-configuration='different-parser'\n"),
    );
    // The selected query directory remains the registered original: authority must
    // still bind the independently supplied measured profile and its settings.
    let error = f.certify_profile(&foreign).unwrap_err().to_string();
    assert!(
        error.contains("registered authority differs from measured canonical profile"),
        "{error}"
    );
    assert_eq!(*f.execution.launches.lock().unwrap(), 0);
}

#[test]
fn identical_profile_toml_cannot_substitute_measured_query_or_helper_bytes() {
    use crate::outbound::suite::interface::SuitePort as _;
    for changed in ["rules/A/query.ryoql", "helper.ryoql"] {
        let f = Fixture::new();
        let foreign = f._root.path().join("measured-foreign");
        copy_measured_profile(&f, &foreign);
        assert_eq!(
            digest(&foreign.join("benchmark.toml")),
            digest(&f.profile.join("benchmark.toml"))
        );
        write(&foreign.join(changed), "substituted measured source bytes");
        let error = f.certify_profile(&foreign).unwrap_err().to_string();
        assert!(
            error.contains("selected rule root differs from measured profile"),
            "{changed}: {error}"
        );
        assert_eq!(*f.execution.launches.lock().unwrap(), 0, "{changed}");

        let suite = crate::outbound::suite::factory::local();
        let tool = suite
            .profile(crate::outbound::suite::types::ArtifactPath(&f.profile))
            .unwrap();
        let error = super::case_inventory::prepare(
            &crate::outbound::inventory::factory::local(),
            &suite,
            &f.registration,
            &f.profile,
            &tool,
            &foreign.join("rules"),
            &f.reference_executable,
            &f.worker_executable,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("selected rule root differs from measured profile"),
            "{changed}: {error}"
        );
        assert_eq!(*f.execution.launches.lock().unwrap(), 0, "{changed}");

        let error = f
            .certify_profile_rules(&foreign, &foreign.join("rules"))
            .unwrap_err()
            .to_string();
        let expected = if changed.ends_with("query.ryoql") {
            "measured query differs from registered context"
        } else {
            "measured helper differs from tested context"
        };
        assert!(error.contains(expected), "{changed}: {error}");
        assert_eq!(*f.execution.launches.lock().unwrap(), 0, "{changed}");
    }
}

#[test]
fn byte_identical_staged_profile_keeps_registered_source_authority() {
    let f = Fixture::new();
    let staged = f._root.path().join("measured-staged");
    copy_measured_profile(&f, &staged);
    assert_eq!(
        digest(&staged.join("benchmark.toml")),
        digest(&f.profile.join("benchmark.toml"))
    );
    assert_ne!(staged, f.profile);
    let certificate = f
        .certify_profile_rules(&staged, &staged.join("rules"))
        .unwrap();
    assert_eq!(certificate.cases().0, 2);
    assert_eq!(*f.execution.launches.lock().unwrap(), 2);
}

#[test]
fn module_inventory_omission_and_addition_refuse_before_native_launch() {
    for change in ["omit-all", "omit-one", "unexpected"] {
        let mut f = Fixture::new();
        let modules = f.document["rules"][0]["modules"].as_object_mut().unwrap();
        match change {
            "omit-all" => modules.clear(),
            "omit-one" => {
                modules.remove("test/second").unwrap();
            }
            "unexpected" => {
                modules.insert("test/unexpected".into(), modules["test/helper"].clone());
            }
            _ => unreachable!(),
        }
        f.save();
        let error = f.certify().unwrap_err().to_string();
        assert!(
            error.contains("registered module inventory differs from pinned context profile"),
            "{change}: {error}"
        );
        assert_eq!(*f.execution.launches.lock().unwrap(), 0, "{change}");
    }
}
