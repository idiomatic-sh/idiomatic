//! Registered certification retains independent artifact namespaces and refuses changed authority.
use super::{interface::CertifyUseCase as _, use_case::CertifyUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::execution::interface::ExecutionPort;
use ryo_benchmark_contracts::domain::measurements::types::*;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;
use ryo_benchmark_contracts::domain::requests::types::CertificationCases;
use serde_json::{Value as Document, json};
use sha2::{Digest as _, Sha256};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

const ARTIFACT: &str = "registered case artifact changed";
const QUERY: &str = "measured query differs";
const MODULE: &str = "measured helper differs";
const RECEIPT_ONLY: &str = "receipt-only inventory cannot authorize execution";
const UNKNOWN_FIELD: &str = "unknown field";
const NAMESPACE: &str = "registered context namespace differs";
const AUTHORITY: &str = "registered authority differs from measured canonical profile";
const IDENTITIES: &str = "case inventory executable identities differ from measured contestants";
const ROOT: &str = "selected rule root differs from measured profile";
const QUERY_CONTEXT: &str = "measured query differs from registered context";
const MODULE_CONTEXT: &str = "measured helper differs from tested context";
const MODULE_INVENTORY: &str = "registered module inventory differs from pinned context profile";

/// How a foreign measured profile reaches certification.
#[derive(Clone, Copy)]
enum Route {
    /// The public operation with the foreign profile as the measured one.
    Certify,
    /// The case inventory preparation alone, with the foreign rules.
    Prepare,
    /// The public operation with the foreign profile and its own rules.
    Rules,
}

#[derive(Clone, Copy)]
enum Scenario {
    /// Both registered contexts execute natively.
    Executes,
    /// The measured contestants are swapped for each other.
    Swapped,
    /// One registered artifact, query, module or authority drifted.
    Drift { change: &'static str, reason: &'static str },
    /// A registered rule is missing from the document.
    MissingRule,
    /// The document or its context substitutes an invocation or the canonical profile.
    Substitution { change: &'static str, reason: &'static str },
    /// The measured profile identity is substituted via the query directory.
    ForeignProfile,
    /// A profile whose `benchmark.toml` is identical but whose source bytes differ.
    Identical { changed: &'static str, route: Route, reason: &'static str },
    /// A byte-identical staged copy of the measured profile.
    Staged,
    /// The registered module inventory omits or adds a module.
    Modules(&'static str),
}

type Input = Scenario;
type Value = usize;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("registered contexts execute and keep colliding relative evidence names separate"),
        word: Word::Succeeds,
        input: || Scenario::Executes,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a byte-identical staged profile keeps registered source authority"),
        word: Word::Succeeds,
        input: || Scenario::Staged,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("swapping distinct measured contestants refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Swapped,
        expect: Expect::Error(Cow::Borrowed(IDENTITIES)),
    },
    Case {
        name: CaseName("a changed registered artifact refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Drift { change: "artifact", reason: ARTIFACT },
        expect: Expect::Error(Cow::Borrowed(ARTIFACT)),
    },
    Case {
        name: CaseName("a changed measured query refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Drift { change: "query", reason: QUERY },
        expect: Expect::Error(Cow::Borrowed(QUERY)),
    },
    Case {
        name: CaseName("a changed measured helper refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Drift { change: "module", reason: MODULE },
        expect: Expect::Error(Cow::Borrowed(MODULE)),
    },
    Case {
        name: CaseName("receipt-only authority refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Drift { change: "receipt-only", reason: RECEIPT_ONLY },
        expect: Expect::Error(Cow::Borrowed(RECEIPT_ONLY)),
    },
    Case {
        name: CaseName("a missing registered rule never issues a certificate"),
        word: Word::Refuses,
        input: || Scenario::MissingRule,
        expect: Expect::Error(Cow::Borrowed(
            super::constants::REGISTERED_INDEPENDENT_CASE_CERTIFICATION_FA,
        )),
    },
    Case {
        name: CaseName("a substituted executable refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "executable", reason: UNKNOWN_FIELD },
        expect: Expect::Error(Cow::Borrowed(UNKNOWN_FIELD)),
    },
    Case {
        name: CaseName("substituted arguments refuse before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "arguments", reason: UNKNOWN_FIELD },
        expect: Expect::Error(Cow::Borrowed(UNKNOWN_FIELD)),
    },
    Case {
        name: CaseName("a substituted directory refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "directory", reason: UNKNOWN_FIELD },
        expect: Expect::Error(Cow::Borrowed(UNKNOWN_FIELD)),
    },
    Case {
        name: CaseName("a substituted environment refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "environment", reason: UNKNOWN_FIELD },
        expect: Expect::Error(Cow::Borrowed(UNKNOWN_FIELD)),
    },
    Case {
        name: CaseName("a substituted context refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "context", reason: NAMESPACE },
        expect: Expect::Error(Cow::Borrowed(NAMESPACE)),
    },
    Case {
        name: CaseName("an extra context file refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "extra-context-file", reason: NAMESPACE },
        expect: Expect::Error(Cow::Borrowed(NAMESPACE)),
    },
    Case {
        name: CaseName("a foreign canonical profile in the registration refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "foreign-profile", reason: AUTHORITY },
        expect: Expect::Error(Cow::Borrowed(AUTHORITY)),
    },
    Case {
        name: CaseName("the measured profile identity cannot be substituted via the query directory"),
        word: Word::Refuses,
        input: || Scenario::ForeignProfile,
        expect: Expect::Error(Cow::Borrowed(AUTHORITY)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured query bytes in certification"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "rules/A/query.ryoql", route: Route::Certify, reason: ROOT },
        expect: Expect::Error(Cow::Borrowed(ROOT)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured query bytes in preparation"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "rules/A/query.ryoql", route: Route::Prepare, reason: ROOT },
        expect: Expect::Error(Cow::Borrowed(ROOT)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured query bytes with foreign rules"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "rules/A/query.ryoql", route: Route::Rules, reason: QUERY_CONTEXT },
        expect: Expect::Error(Cow::Borrowed(QUERY_CONTEXT)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured helper bytes in certification"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "helper.ryoql", route: Route::Certify, reason: ROOT },
        expect: Expect::Error(Cow::Borrowed(ROOT)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured helper bytes in preparation"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "helper.ryoql", route: Route::Prepare, reason: ROOT },
        expect: Expect::Error(Cow::Borrowed(ROOT)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured helper bytes with foreign rules"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "helper.ryoql", route: Route::Rules, reason: MODULE_CONTEXT },
        expect: Expect::Error(Cow::Borrowed(MODULE_CONTEXT)),
    },
    Case {
        name: CaseName("omitting every registered module refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Modules("omit-all"),
        expect: Expect::Error(Cow::Borrowed(MODULE_INVENTORY)),
    },
    Case {
        name: CaseName("omitting one registered module refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Modules("omit-one"),
        expect: Expect::Error(Cow::Borrowed(MODULE_INVENTORY)),
    },
    Case {
        name: CaseName("adding an unexpected module refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Modules("unexpected"),
        expect: Expect::Error(Cow::Borrowed(MODULE_INVENTORY)),
    },
];

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
/// Marks the files executable; a file system with no modes needs no marking.
#[cfg(unix)]
fn make_executable(files: &[&Path]) {
    for executable in files {
        std::fs::set_permissions(
            executable,
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .unwrap();
    }
}
#[cfg(not(unix))]
fn make_executable(_files: &[&Path]) {}
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
    document: Document,
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

/// The refusal named by `reason` when the error says it, else the error in full; no contestant ran beyond `launches`.
fn refusal(f: &Fixture, error: &ReportsError, reason: &'static str, launches: usize) -> Failure {
    assert_eq!(*f.execution.launches.lock().unwrap(), launches, "{error}");
    let text = error.to_string();
    if text.contains(reason) {
        Cow::Borrowed(reason)
    } else {
        Cow::Owned(text)
    }
}

fn failed(error: &ReportsError) -> Failure {
    Cow::Owned(error.to_string())
}

fn executes() -> Result<Value, Failure> {
    let f = Fixture::new();
    assert_ne!(f.reference_executable, f.worker_executable);
    assert_ne!(
        digest(&f.reference_executable),
        digest(&f.worker_executable)
    );
    let certificate = f.certify().map_err(|error| failed(&error))?;
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
    let launches = *f.execution.launches.lock().unwrap();
    Ok(launches)
}

fn swapped() -> Result<Value, Failure> {
    use crate::outbound::suite::interface::SuitePort as _;
    let f = Fixture::new();
    let suite = crate::outbound::suite::factory::local();
    let tool = suite
        .profile(crate::outbound::suite::types::ArtifactPath(&f.profile))
        .unwrap();
    super::case_inventory::prepare(
        &crate::outbound::inventory::factory::local(),
        &suite,
        &f.registration,
        &f.profile,
        &tool,
        &f.profile.join("rules"),
        &f.worker_executable,
        &f.reference_executable,
    )
    .map(|(_, paths)| paths.len())
    .map_err(|error| refusal(&f, &error, IDENTITIES, 0))
}

fn drift(change: &str, reason: &'static str) -> Result<Value, Failure> {
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

fn missing_rule() -> Result<Value, Failure> {
    let mut f = Fixture::new();
    f.document["rules"].as_array_mut().unwrap().pop();
    f.save();
    f.certify().map(|_| 0).map_err(|error| {
        refusal(
            &f,
            &error,
            super::constants::REGISTERED_INDEPENDENT_CASE_CERTIFICATION_FA,
            1,
        )
    })
}

fn substitution(change: &str, reason: &'static str) -> Result<Value, Failure> {
    let mut f = Fixture::new();
    match change {
        "executable" => f.document["rules"][0]["command"]["executable"] = json!("foreign-worker"),
        "arguments" => {
            f.document["rules"][0]["command"]["arguments"] = json!(["different-operation"])
        }
        "directory" => f.document["rules"][0]["command"]["directory"] = json!("foreign-context"),
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
    f.certify()
        .map(|_| 0)
        .map_err(|error| refusal(&f, &error, reason, 0))
}

fn foreign_profile() -> Result<Value, Failure> {
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
    f.certify_profile(&foreign)
        .map(|_| 0)
        .map_err(|error| refusal(&f, &error, AUTHORITY, 0))
}

fn identical(changed: &str, route: Route, reason: &'static str) -> Result<Value, Failure> {
    use crate::outbound::suite::interface::SuitePort as _;
    let f = Fixture::new();
    let foreign = f._root.path().join("measured-foreign");
    copy_measured_profile(&f, &foreign);
    assert_eq!(
        digest(&foreign.join("benchmark.toml")),
        digest(&f.profile.join("benchmark.toml"))
    );
    write(&foreign.join(changed), "substituted measured source bytes");
    match route {
        Route::Certify => f.certify_profile(&foreign).map(|_| 0),
        Route::Prepare => {
            let suite = crate::outbound::suite::factory::local();
            let tool = suite
                .profile(crate::outbound::suite::types::ArtifactPath(&f.profile))
                .unwrap();
            super::case_inventory::prepare(
                &crate::outbound::inventory::factory::local(),
                &suite,
                &f.registration,
                &f.profile,
                &tool,
                &foreign.join("rules"),
                &f.reference_executable,
                &f.worker_executable,
            )
            .map(|(_, paths)| paths.len())
        }
        Route::Rules => f
            .certify_profile_rules(&foreign, &foreign.join("rules"))
            .map(|_| 0),
    }
    .map_err(|error| refusal(&f, &error, reason, 0))
}

fn staged() -> Result<Value, Failure> {
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
        .map_err(|error| failed(&error))?;
    assert_eq!(*f.execution.launches.lock().unwrap(), 2);
    Ok(certificate.cases().0)
}

fn modules(change: &str) -> Result<Value, Failure> {
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
        .map_err(|error| refusal(&f, &error, MODULE_INVENTORY, 0))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Executes => executes(),
        Scenario::Swapped => swapped(),
        Scenario::Drift { change, reason } => drift(change, reason),
        Scenario::MissingRule => missing_rule(),
        Scenario::Substitution { change, reason } => substitution(change, reason),
        Scenario::ForeignProfile => foreign_profile(),
        Scenario::Identical { changed, route, reason } => identical(changed, route, reason),
        Scenario::Staged => staged(),
        Scenario::Modules(change) => modules(change),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
