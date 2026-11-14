//! The clippy adapter of the linter port: the shared contract run through it, then its invocation, its environment, the case
//! manifest and the source marks its preparation leaves, and the decoding of a message stream clippy 0.1.100 printed.
#![cfg(test)]
use super::{adapter::LinterClippyAdapter, settings::LinterClippySettings};
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::outbound::linter::interface::LinterPort as _;
use ryo_benchmark::outbound::suite::{interface::SuitePort as _, types::ArtifactPath};
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, Invocation, ProcessExitCode, ProcessOutput, ResourceObservation,
    TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, RuleContextPreparation, RunRequest,
};
use std::borrow::Cow;
use std::path::Path;

/// `pub const X: u32 = 0123;` in `src/lib.rs`, checked with `-W clippy::zero_prefixed_literal`: the library build and its test
/// build each print the finding.
const ZERO_PREFIXED: &str = r#"{"reason":"compiler-message","message":{"message":"this is a decimal constant","code":{"code":"clippy::zero_prefixed_literal"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":19,"byte_end":23,"is_primary":true}]}}
{"reason":"compiler-message","message":{"message":"this is a decimal constant","code":{"code":"clippy::zero_prefixed_literal"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":19,"byte_end":23,"is_primary":true}]}}
{"reason":"build-finished","success":true}"#;
/// A project's own manifest, which preparation keeps byte for byte.
const PROJECT_MANIFEST: &str =
    "[package]\nname = \"project\"\nversion = \"1.2.3\"\nedition = \"2021\"\n";

/// What one row calls on the adapter.
#[derive(Clone, Copy, Debug)]
enum Call {
    /// The shared linter contract.
    Contract,
    /// The version line of the committed profile.
    VersionLine,
    /// Prepare the named files of a case root, rendered as the executable and its arguments.
    Prepare(&'static [&'static str]),
    /// Prepare one file, then name the `-A` arguments that come before the first `-W`.
    AllowedBeforeSelection,
    /// Prepare one file, then render the invocation's environment, one `name=value` a line.
    Environment,
    /// Prepare one file of a case root that has no manifest, then read the manifest preparation wrote.
    CaseManifest,
    /// Prepare one file of a root that has its own manifest, then read that manifest.
    ProjectManifest,
    /// Prepare one file last modified long ago, then say whether it is now marked modified since the instant before.
    SourceMarked,
    /// Prepare a request that carries a rule configuration.
    RuleContext,
    /// Decode what a run of `src/lib.rs` printed with this exit status.
    Decode(i32, &'static str),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Invalid,
    Unexpected,
}
type Input = Call;
type Value = Cow<'static, str>;
type Failure = Refusal;

/// The rows of what is specific to this adapter: a name, one case word, a call, and its rendering or refusal.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("concrete clippy answers the shared linter contract"),
        word: Word::Succeeds,
        input: || Call::Contract,
        expect: Expect::Value(Cow::Borrowed("contract held")),
    },
    Case {
        name: CaseName(
            "version_line_is_clippy_spelling: the committed profile's line is the one clippy-driver prints",
        ),
        word: Word::Succeeds,
        input: || Call::VersionLine,
        expect: Expect::Value(Cow::Borrowed("clippy 0.1.100 (0ef1bc3860 2026-11-09)")),
    },
    Case {
        name: CaseName(
            "selection_follows_the_blanket_allow: the toolchain's clippy command checks the workspace offline, every lint off, then each selected one warned",
        ),
        word: Word::Succeeds,
        input: || Call::Prepare(&["src/lib.rs", "src/a.rs"]),
        expect: Expect::Value(Cow::Borrowed(
            "toolchain/bin/cargo-clippy in root: clippy --workspace --lib --bins --tests --examples --message-format=json --offline -- -A clippy::all -A clippy::pedantic -A clippy::nursery -A clippy::restriction -A clippy::cargo -W clippy::zero_prefixed_literal",
        )),
    },
    Case {
        name: CaseName(
            "every_group_is_allowed_before_the_selection: a lint a project's manifest turns on stays off",
        ),
        word: Word::Succeeds,
        input: || Call::AllowedBeforeSelection,
        expect: Expect::Value(Cow::Borrowed(
            "clippy::all clippy::cargo clippy::nursery clippy::pedantic clippy::restriction",
        )),
    },
    Case {
        name: CaseName(
            "environment_pins_the_toolchain: the siblings of the verified driver, the shared targets as target and configuration directory, one job",
        ),
        word: Word::Succeeds,
        input: || Call::Environment,
        expect: Expect::Value(Cow::Borrowed(
            "CARGO=toolchain/bin/cargo\nCARGO_BUILD_JOBS=1\nCARGO_TARGET_DIR=targets\nCLIPPY_CONF_DIR=targets\nRAYON_NUM_THREADS=1\nRUSTC=toolchain/bin/rustc",
        )),
    },
    Case {
        name: CaseName("a case root without a manifest becomes the case crate"),
        word: Word::Succeeds,
        input: || Call::CaseManifest,
        expect: Expect::Value(Cow::Borrowed(
            "[package]\nname = \"case\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
        )),
    },
    Case {
        name: CaseName("project_manifest_is_kept: a root's own manifest is never written"),
        word: Word::Succeeds,
        input: || Call::ProjectManifest,
        expect: Expect::Value(Cow::Borrowed(PROJECT_MANIFEST)),
    },
    Case {
        name: CaseName(
            "prepare_marks_sources_modified: the check runs again instead of replaying cached messages",
        ),
        word: Word::Succeeds,
        input: || Call::SourceMarked,
        expect: Expect::Value(Cow::Borrowed("marked")),
    },
    Case {
        name: CaseName("decode answers the selected defaults, the library and test builds once"),
        word: Word::Succeeds,
        input: || Call::Decode(0, ZERO_PREFIXED),
        expect: Expect::Value(Cow::Borrowed("zero_prefixed_literal 19..23")),
    },
    Case {
        name: CaseName("an exit other than 0 or 101 is an operational failure"),
        word: Word::Refuses,
        input: || Call::Decode(2, ZERO_PREFIXED),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("an empty file list names nothing to check"),
        word: Word::Refuses,
        input: || Call::Prepare(&[]),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a rule configuration is refused: this adapter projects none"),
        word: Word::Refuses,
        input: || Call::RuleContext,
        expect: Expect::Error(Refusal::Invalid),
    },
];

fn refusal(error: &Error) -> Refusal {
    match error {
        Error::Invalid(_) => Refusal::Invalid,
        Error::Io(_) | Error::Json(_) | Error::Toml(_) => Refusal::Unexpected,
    }
}
fn adapter() -> Result<LinterClippyAdapter, Failure> {
    Ok(LinterClippyAdapter::new(LinterClippySettings {
        executable: "toolchain/bin/clippy-driver".into(),
        defaults: [RuleId::try_from("zero_prefixed_literal").map_err(|_blank| Refusal::Invalid)?]
            .into_iter()
            .collect(),
        targets: "targets".into(),
    }))
}
/// A request of the named files of `root` that runs the pinned defaults.
fn request(files: &[&str], root: &Path) -> RunRequest {
    RunRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Frozen,
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots:
            ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata:
            ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: root.to_owned(),
        state: root.join("state"),
        files: files
            .iter()
            .map(|file| RelativePath((*file).into()))
            .collect(),
    }
}
/// A case root holding each named file, empty.
fn case_root(files: &[&str]) -> Result<tempfile::TempDir, Failure> {
    let root = tempfile::tempdir().map_err(|_error| Refusal::Unexpected)?;
    for file in files {
        let path = root.path().join(file);
        std::fs::create_dir_all(path.parent().ok_or(Refusal::Unexpected)?)
            .map_err(|_error| Refusal::Unexpected)?;
        std::fs::write(&path, "").map_err(|_error| Refusal::Unexpected)?;
    }
    Ok(root)
}
/// Prepare `src/lib.rs` of a fresh case root and hand back the root with the invocation.
fn prepared() -> Result<(tempfile::TempDir, Invocation), Failure> {
    let root = case_root(&["src/lib.rs"])?;
    let invocation = adapter()?
        .prepare(&request(&["src/lib.rs"], root.path()))
        .map_err(|error| refusal(&error))?;
    Ok((root, invocation))
}

/// Calls the adapter on one input and renders what it answered.
fn subject(input: Input) -> Result<Value, Failure> {
    let adapter = adapter()?;
    match input {
        Call::Contract => {
            ryo_benchmark::outbound::linter::fixtures::contract::check(&adapter);
            Ok(Cow::Borrowed("contract held"))
        }
        Call::VersionLine => {
            let tool = ryo_benchmark::outbound::suite::factory::local()
                .profile(ArtifactPath(Path::new(env!("CARGO_MANIFEST_DIR"))))
                .map_err(|error| refusal(&error))?;
            Ok(adapter.version_line(&tool).0.into_string().into())
        }
        Call::Prepare(files) => {
            let root = case_root(files)?;
            let invocation = adapter
                .prepare(&request(files, root.path()))
                .map_err(|error| refusal(&error))?;
            Ok(format!(
                "{} in {}: {}",
                invocation.executable.display(),
                invocation.directory.display(),
                invocation
                    .arguments
                    .iter()
                    .map(|argument| argument.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .replace(&*root.path().to_string_lossy(), "root")
            .into())
        }
        Call::AllowedBeforeSelection => {
            let (_root, invocation) = prepared()?;
            let first_selection = invocation
                .arguments
                .iter()
                .position(|argument| argument == "-W")
                .ok_or(Refusal::Unexpected)?;
            let mut allowed = invocation.arguments[..first_selection]
                .windows(2)
                .filter(|pair| pair[0] == "-A")
                .map(|pair| pair[1].to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            allowed.sort();
            Ok(allowed.join(" ").into())
        }
        Call::Environment => {
            let (_root, invocation) = prepared()?;
            Ok(invocation
                .environment
                .iter()
                .map(|(name, value)| {
                    format!("{}={}", name.to_string_lossy(), value.to_string_lossy())
                })
                .collect::<Vec<_>>()
                .join("\n")
                .into())
        }
        Call::CaseManifest => {
            let (root, _invocation) = prepared()?;
            Ok(std::fs::read_to_string(root.path().join("Cargo.toml"))
                .map_err(|_error| Refusal::Unexpected)?
                .into())
        }
        Call::ProjectManifest => {
            let root = case_root(&["src/lib.rs"])?;
            let manifest = root.path().join("Cargo.toml");
            std::fs::write(&manifest, PROJECT_MANIFEST).map_err(|_error| Refusal::Unexpected)?;
            let _invocation = adapter
                .prepare(&request(&["src/lib.rs"], root.path()))
                .map_err(|error| refusal(&error))?;
            Ok(std::fs::read_to_string(&manifest)
                .map_err(|_error| Refusal::Unexpected)?
                .into())
        }
        Call::SourceMarked => {
            let root = case_root(&["src/lib.rs"])?;
            let source = root.path().join("src/lib.rs");
            std::fs::File::options()
                .write(true)
                .open(&source)
                .and_then(|file| file.set_modified(std::time::SystemTime::UNIX_EPOCH))
                .map_err(|_error| Refusal::Unexpected)?;
            let before = std::time::SystemTime::now();
            let _invocation = adapter
                .prepare(&request(&["src/lib.rs"], root.path()))
                .map_err(|error| refusal(&error))?;
            let modified = std::fs::metadata(&source)
                .and_then(|metadata| metadata.modified())
                .map_err(|_error| Refusal::Unexpected)?;
            Ok(Cow::Borrowed(if modified >= before {
                "marked"
            } else {
                "unmarked"
            }))
        }
        Call::RuleContext => {
            let root = case_root(&["src/lib.rs"])?;
            let mut prepared = request(&["src/lib.rs"], root.path());
            prepared.rule_context = RuleContextPreparation::Prepared(
                ryo_benchmark_contracts::domain::context::types::PreparedRuleContext {
                    tables: ryo_benchmark_contracts::domain::context::types::RuleContextTables {
                        selected: ryo_benchmark_contracts::domain::context::types::TableName(
                            "selected".into(),
                        ),
                        disabled: ryo_benchmark_contracts::domain::context::types::TableName(
                            "disabled".into(),
                        ),
                        external: ryo_benchmark_contracts::domain::context::types::TableName(
                            "external".into(),
                        ),
                    },
                    configuration:
                        ryo_benchmark_contracts::domain::context::types::RuleConfiguration {
                            external_prefixes: Vec::new(),
                            disabled_rules: Default::default(),
                        },
                },
            );
            adapter
                .prepare(&prepared)
                .map(|_invocation| Cow::Borrowed("prepared"))
                .map_err(|error| refusal(&error))
        }
        Call::Decode(status, stdout) => {
            let report = adapter
                .decode(
                    &request(&["src/lib.rs"], Path::new("root")),
                    ProcessOutput {
                        signal: TerminationSignal::Exited,
                        status: ProcessExitCode(status),
                        stdout: CapturedBytes(stdout.as_bytes().to_vec()),
                        stderr: CapturedBytes(Vec::new()),
                        wall: Default::default(),
                        resources: ResourceObservation::WallOnly,
                    },
                )
                .map_err(|error| refusal(&error))?;
            Ok(report
                .diagnostics
                .iter()
                .map(|finding| {
                    format!(
                        "{} {}..{}",
                        finding.rule.as_ref(),
                        finding.start.0,
                        finding.end.0
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
                .into())
        }
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
