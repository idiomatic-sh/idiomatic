//! Registered certification retains independent artifact namespaces and refuses changed authority.
use super::{interface::CertifyUseCase as _, use_case::CertifyUseCaseImpl};
use crate::domain::suite::types::{BenchmarkCoverage, CertificationCases};
use crate::domain::{diagnostics::errors::DiagnosticsError, measurements::types::*};
use crate::outbound::execution::interface::ExecutionPort;
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
    fn run(&self, request: Invocation) -> Result<ProcessOutput, DiagnosticsError> {
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
    executable: PathBuf,
    registration: PathBuf,
    document: Value,
    execution: Arc<Native>,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let profile = root.path().join("measured");
        let executable = root.path().join(super::constants::FIXTURE_TOOL_FILE);
        write(&executable, super::constants::FIXTURE_TOOL_SCRIPT);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        write(&profile.join("helper.ryoql"), "shared tested helper");
        write(
            &profile.join("benchmark.toml"),
            &format!(
                "tool='fixture'\nversion='1'\nsha256=['{}']\ndefaults=['A','B']\nquery-modules={{'test/helper'='helper.ryoql'}}\n",
                digest(&executable)
            ),
        );
        let entries = ["A","B"].map(|rule| {
            write(&profile.join(format!("rules/{rule}/query.ryoql")), &format!("query {rule}"));
            write(&profile.join(format!("rules/{rule}/cases.toml")), "[[cases]]\nname='positive'\nfiles={'input.txt'='x'}\nexpected=[{file='input.txt',start=0,end=1}]\n[[cases]]\nname='negative'\nfiles={'input.txt'='y'}\nexpected=[]\n");
            let context = root.path().join(format!("registered/{rule}"));
            let definition = context.join(format!("rules/{rule}/query.ryoql"));
            let helper = context.join("helper.ryoql");
            let evidence = context.join("evidence/oracle.json");
            write(&definition,&format!("query {rule}")); write(&helper,"shared tested helper"); write(&evidence,&format!("independent {rule} evidence"));
            write(&context.join("benchmark.toml"), &format!("tool='fixture'\nversion='1'\nsha256=['{}']\ndefaults=['{rule}']\nquery-modules={{'test/helper'='helper.ryoql'}}\n",digest(&executable)));
            let artifacts = [definition.clone(),helper.clone(),evidence,context.join("benchmark.toml")].into_iter().map(|p|(p.display().to_string(),json!(digest(&p)))).collect::<serde_json::Map<_,_>>();
            json!({"rule_id":rule,"reason":null,"instance":null,"definition":{"path":definition,"sha256":digest(&definition)},"modules":{"test/helper":{"path":helper,"sha256":digest(&helper)}},"artifacts":artifacts,
                "command":{"kind":"case-verification","context":context},
                "cases":[{"rule":rule,"name":"independent","source_sha256":"source","files":["input.txt"],"expected":[],"project_context_mode":"isolated","reference_discrepancy":null}]})
        });
        let document = json!({"authority":{"kind":"execution","profile":profile,"profile_sha256":digest(&profile.join("benchmark.toml")),"reference":{"path":executable,"sha256":digest(&executable)},"worker":{"path":executable,"sha256":digest(&executable)}},"provider":"neutral","defaults":["A","B"],"rules":entries,"identity":{}});
        let registration = root.path().join("registration.json");
        std::fs::write(&registration, serde_json::to_vec(&document).unwrap()).unwrap();
        Self {
            _root: root,
            profile,
            executable: executable.clone(),
            registration,
            document,
            execution: Arc::new(Native {
                launches: Mutex::new(0),
                worker: executable.clone(),
                reference: executable.clone(),
            }),
        }
    }
    fn certify(
        &self,
    ) -> Result<crate::domain::suite::types::certificate::value::Certificate, DiagnosticsError>
    {
        let operation = CertifyUseCaseImpl::new(
            Arc::new(crate::outbound::suite::factory::local()),
            Arc::new(crate::outbound::linter::fixtures::fake::LinterFake::default()),
            Arc::new(crate::outbound::linter::fixtures::fake::LinterFake::default()),
            self.execution.clone(),
            Arc::new(crate::outbound::inventory::factory::local()),
        );
        operation.certify(
            &self.profile,
            &self.profile,
            &self.executable,
            &self.executable,
            &self.profile.join("rules"),
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
#[test]
fn registered_contexts_execute_and_keep_colliding_relative_evidence_names_separate() {
    let f = Fixture::new();
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
