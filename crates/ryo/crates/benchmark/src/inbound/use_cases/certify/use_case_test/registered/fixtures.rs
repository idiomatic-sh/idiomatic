//! Test support of the registered certification proofs: the native execution fake and the registration fixture.
use crate::inbound::use_cases::certify::interface::CertifyUseCase as _;
use crate::outbound::execution::interface::ExecutionPort;
use ryo_benchmark_contracts::domain::measurements::types::*;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;
use ryo_benchmark_contracts::domain::requests::types::CertificationCases;
use ryo_benchmark_contracts::domain::requests::types::RegisteredCoverage;
use serde_json::{Value as Document, json};
use sha2::{Digest as _, Sha256};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

pub(in crate::inbound::use_cases::certify::use_case_test) struct Native {
    pub(in crate::inbound::use_cases::certify::use_case_test) launches: Mutex<usize>,
    pub(in crate::inbound::use_cases::certify::use_case_test) worker: PathBuf,
    pub(in crate::inbound::use_cases::certify::use_case_test) reference: PathBuf,
    /// The registered rule whose native verification fails, if any.
    pub(in crate::inbound::use_cases::certify::use_case_test) failing: Mutex<Option<&'static str>>,
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
            "rule":rule,"name":"independent","side":side,"complete":true,"error":null,"documented_reference_discrepancy":false,
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
        let failed = *self.failing.lock().unwrap() == Some(rule);
        Ok(ProcessOutput {
            status: ProcessExitCode(i32::from(failed)),
            signal: ryo_benchmark_contracts::domain::measurements::types::TerminationSignal::Exited,
            stdout: CapturedBytes(vec![]),
            stderr: CapturedBytes(vec![]),
            wall: Duration::ZERO,
            resources: ResourceObservation::WallOnly,
        })
    }
}
/// Marks the files executable; a file system with no modes needs no marking.
#[cfg(unix)]
pub(in crate::inbound::use_cases::certify::use_case_test) fn make_executable(files: &[&Path]) {
    for executable in files {
        std::fs::set_permissions(
            executable,
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .unwrap();
    }
}
#[cfg(not(unix))]
pub(in crate::inbound::use_cases::certify::use_case_test) fn make_executable(_files: &[&Path]) {}
pub(in crate::inbound::use_cases::certify::use_case_test) fn digest(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}
pub(in crate::inbound::use_cases::certify::use_case_test) fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}
pub(in crate::inbound::use_cases::certify::use_case_test) struct Fixture {
    pub(in crate::inbound::use_cases::certify::use_case_test) _root: tempfile::TempDir,
    pub(in crate::inbound::use_cases::certify::use_case_test) profile: PathBuf,
    pub(in crate::inbound::use_cases::certify::use_case_test) reference_executable: PathBuf,
    pub(in crate::inbound::use_cases::certify::use_case_test) worker_executable: PathBuf,
    pub(in crate::inbound::use_cases::certify::use_case_test) registration: PathBuf,
    pub(in crate::inbound::use_cases::certify::use_case_test) document: Document,
    pub(in crate::inbound::use_cases::certify::use_case_test) execution: Arc<Native>,
}
impl Fixture {
    pub(in crate::inbound::use_cases::certify::use_case_test) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let profile = root.path().join("measured");
        let reference_executable = root
            .path()
            .join(crate::inbound::use_cases::certify::constants::FIXTURE_TOOL_FILE);
        let worker_executable = root.path().join("independent-worker");
        write(
            &reference_executable,
            crate::inbound::use_cases::certify::constants::FIXTURE_TOOL_SCRIPT,
        );
        write(
            &worker_executable,
            "#!/bin/sh\nprintf 'independent worker\\n'\n",
        );
        assert_ne!(digest(&reference_executable), digest(&worker_executable));
        make_executable(&[&reference_executable, &worker_executable]);
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
                "cases":[{"rule":rule,"name":"independent","case_sha256":"case-pin","comparison":"rule","source_sha256":"source","files":["input.txt"],"expected":[],"project_context_mode":"isolated","reference_discrepancy":null}]})
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
                failing: Mutex::new(None),
            }),
        }
    }
    pub(in crate::inbound::use_cases::certify::use_case_test) fn certify(
        &self,
    ) -> Result<crate::domain::suite::types::certificate::value::Certificate, ReportsError> {
        self.certify_profile(&self.profile)
    }
    pub(in crate::inbound::use_cases::certify::use_case_test) fn certify_profile(
        &self,
        measured_profile: &Path,
    ) -> Result<crate::domain::suite::types::certificate::value::Certificate, ReportsError> {
        self.certify_profile_rules(measured_profile, &self.profile.join("rules"))
    }
    pub(in crate::inbound::use_cases::certify::use_case_test) fn certify_profile_rules(
        &self,
        measured_profile: &Path,
        selected_rules: &Path,
    ) -> Result<crate::domain::suite::types::certificate::value::Certificate, ReportsError> {
        self.certify_as(
            measured_profile,
            selected_rules,
            BenchmarkCoverage::Accounted,
            RegisteredCoverage::Full,
        )
    }
    /// Certify the measured profile under `scope`, proving the registered rules `coverage` names.
    pub(in crate::inbound::use_cases::certify::use_case_test) fn certify_as(
        &self,
        measured_profile: &Path,
        selected_rules: &Path,
        scope: BenchmarkCoverage,
        coverage: RegisteredCoverage,
    ) -> Result<crate::domain::suite::types::certificate::value::Certificate, ReportsError> {
        let operation = crate::inbound::use_cases::certify::use_case::CertifyUseCaseImpl::new(
            Arc::new(crate::outbound::suite::factory::local()),
            Arc::new(crate::outbound::receipts::fixtures::fake::ReceiptsFake::default()),
            Arc::new(crate::outbound::context::factory::local()),
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
            scope,
            &CertificationCases::Registered {
                registration: self.registration.clone(),
                output: self._root.path().join("receipts"),
                coverage,
            },
        )
    }
    pub(in crate::inbound::use_cases::certify::use_case_test) fn save(&self) {
        std::fs::write(
            &self.registration,
            serde_json::to_vec(&self.document).unwrap(),
        )
        .unwrap();
    }
}
pub(in crate::inbound::use_cases::certify::use_case_test) fn copy_measured_profile(
    fixture: &Fixture,
    destination: &Path,
) {
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

/// The refusal named by `reason` when the error says it, else the error in full; no contestant ran beyond `launches`.
pub(in crate::inbound::use_cases::certify::use_case_test) fn refusal(
    f: &Fixture,
    error: &ReportsError,
    reason: &'static str,
    launches: usize,
) -> Cow<'static, str> {
    assert_eq!(*f.execution.launches.lock().unwrap(), launches, "{error}");
    let text = error.to_string();
    if text.contains(reason) {
        Cow::Borrowed(reason)
    } else {
        Cow::Owned(text)
    }
}

pub(in crate::inbound::use_cases::certify::use_case_test) fn failed(
    error: &ReportsError,
) -> Cow<'static, str> {
    Cow::Owned(error.to_string())
}

pub(in crate::inbound::use_cases::certify::use_case_test) fn swapped(
    reason: &'static str,
) -> Result<usize, Cow<'static, str>> {
    use crate::outbound::suite::interface::SuitePort as _;
    let f = Fixture::new();
    let suite = crate::outbound::suite::factory::local();
    let tool = suite
        .profile(crate::outbound::suite::types::ArtifactPath(&f.profile))
        .unwrap();
    crate::inbound::use_cases::certify::service::prepare(
        &crate::outbound::inventory::factory::local(),
        &suite,
        &f.registration,
        &f.profile,
        &tool,
        &f.profile.join("rules"),
        &f.worker_executable,
        &f.reference_executable,
        &RegisteredCoverage::Full,
    )
    .map(|context| context.paths.len())
    .map_err(|error| refusal(&f, &error, reason, 0))
}

pub(in crate::inbound::use_cases::certify::use_case_test) fn drift(
    change: &str,
    reason: &'static str,
) -> Result<usize, Cow<'static, str>> {
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
    f.certify()
        .map(|_| 0)
        .map_err(|error| refusal(&f, &error, reason, 0))
}

pub(in crate::inbound::use_cases::certify::use_case_test) fn missing_rule()
-> Result<usize, Cow<'static, str>> {
    let mut f = Fixture::new();
    f.document["rules"].as_array_mut().unwrap().pop();
    f.save();
    f.certify().map(|_| 0).map_err(|error| {
        refusal(
            &f,
            &error,
            crate::application::inventory_verification::constants::REGISTERED_INDEPENDENT_CASE_CERTIFICATION_FA,
            1,
        )
    })
}

pub(in crate::inbound::use_cases::certify::use_case_test) fn modules(
    change: &str,
    reason: &'static str,
) -> Result<usize, Cow<'static, str>> {
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
    f.certify()
        .map(|_| 0)
        .map_err(|error| refusal(&f, &error, reason, 0))
}

/// A profile staged from the registered one with only `rules` as its defaults, as a diagnostic run stages it.
pub(in crate::inbound::use_cases::certify::use_case_test) fn narrowed(
    fixture: &Fixture,
    rules: &[&str],
) -> PathBuf {
    let destination = fixture._root.path().join("narrowed");
    copy_measured_profile(fixture, &destination);
    let manifest = std::fs::read_to_string(destination.join("benchmark.toml")).unwrap();
    let defaults = rules
        .iter()
        .map(|rule| format!("'{rule}'"))
        .collect::<Vec<_>>()
        .join(",");
    write(
        &destination.join("benchmark.toml"),
        &manifest.replace("defaults=['A','B']", &format!("defaults=[{defaults}]")),
    );
    for rule in ["A", "B"] {
        if !rules.contains(&rule) {
            std::fs::remove_dir_all(destination.join("rules").join(rule)).unwrap();
        }
    }
    destination
}

/// Certify the `measured` profile under `scope` with rule A selected from `source`, while registered rule B fails.
pub(in crate::inbound::use_cases::certify::use_case_test) fn selected(
    f: &Fixture,
    measured: &Path,
    scope: BenchmarkCoverage,
    source: &Path,
) -> Result<crate::domain::suite::types::certificate::value::Certificate, ReportsError> {
    *f.execution.failing.lock().unwrap() = Some("B");
    f.certify_as(
        measured,
        &measured.join("rules"),
        scope,
        RegisteredCoverage::Selected {
            rules: vec![ryo::linter::RuleId::try_from("A").unwrap()],
            source: source.to_owned(),
        },
    )
}
