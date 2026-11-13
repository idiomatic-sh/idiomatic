//! The oxlint adapter of the linter port: the shared contract run through it, then its invocation, exit statuses and decoding
//! of a document oxlint 1.76.0 printed.
#![cfg(test)]
use super::{adapter::LinterOxlintAdapter, settings::LinterOxlintSettings};
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::outbound::linter::interface::LinterPort as _;
use ryo_benchmark::outbound::suite::{interface::SuitePort as _, types::ArtifactPath};
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ProcessExitCode, ProcessOutput, ResourceObservation, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use std::borrow::Cow;

/// `class C { m() { const self = this; return self } }` linted with `-D typescript/no-this-alias`.
const ALIAS: &str = r#"{ "diagnostics": [{"message": "Unexpected aliasing of 'this' to local variable.","code": "typescript(no-this-alias)","severity": "error","causes": [],"url": "https://oxc.rs/docs/guide/usage/linter/rules/typescript/no-this-alias.html","help": "Assigning a variable to this instead of properly using arrow lambdas may be a symptom of pre-ES2015 practices or not managing scope well.","filename": "alias.ts","labels": [{"span": {"offset": 22,"length": 4,"line": 1,"column": 23}}],"related": []}],
              "number_of_files": 1,
              "number_of_rules": 1,
              "threads_count": 1,
              "start_time": 0.206509685
            }"#;

/// What one row calls on the adapter.
#[derive(Clone, Copy, Debug)]
enum Call {
    /// The shared linter contract.
    Contract,
    /// The version line of the committed profile.
    VersionLine,
    /// The invocation over the named files, rendered as its arguments, the request's state spelled `state`.
    Prepare(&'static [&'static str]),
    /// Prepare one file, then read the file the invocation names after `-c`.
    Configuration,
    /// Prepare one file, then decode what a run of it printed with this exit status.
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
        name: CaseName("concrete oxlint answers the shared linter contract"),
        word: Word::Succeeds,
        input: || Call::Contract,
        expect: Expect::Value(Cow::Borrowed("contract held")),
    },
    Case {
        name: CaseName(
            "version_line_is_oxlint_spelling: the committed profile's line is the one oxlint prints",
        ),
        word: Word::Succeeds,
        input: || Call::VersionLine,
        expect: Expect::Value(Cow::Borrowed("Version: 1.76.0")),
    },
    Case {
        name: CaseName(
            "each selected rule is denied alone, over the listed files, in the request's root",
        ),
        word: Word::Succeeds,
        input: || Call::Prepare(&["src/a.ts", "b.tsx"]),
        expect: Expect::Value(Cow::Borrowed(
            "root: -c state/oxlint.json -A all -D typescript/no-this-alias --format json --threads 1 --disable-nested-config -- src/a.ts b.tsx",
        )),
    },
    Case {
        name: CaseName(
            "the run passes -c to an empty configuration in its state, so a project's own oxlint configuration is never read",
        ),
        word: Word::Succeeds,
        input: || Call::Configuration,
        expect: Expect::Value(Cow::Borrowed("{}")),
    },
    Case {
        name: CaseName("exit_one_with_findings_decodes"),
        word: Word::Succeeds,
        input: || Call::Decode(1, ALIAS),
        expect: Expect::Value(Cow::Borrowed("typescript.no-this-alias 22..26")),
    },
    Case {
        name: CaseName("an exit other than 0 or 1 is an operational failure"),
        word: Word::Refuses,
        input: || Call::Decode(2, ALIAS),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("exit 0 with findings contradicts itself"),
        word: Word::Refuses,
        input: || Call::Decode(0, ALIAS),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("an empty file list cannot walk the directory"),
        word: Word::Refuses,
        input: || Call::Prepare(&[]),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a file named twice is refused"),
        word: Word::Refuses,
        input: || Call::Prepare(&["a.ts", "a.ts"]),
        expect: Expect::Error(Refusal::Invalid),
    },
];

fn refusal(error: &Error) -> Refusal {
    match error {
        Error::Invalid(_) => Refusal::Invalid,
        Error::Io(_) | Error::Json(_) | Error::Toml(_) => Refusal::Unexpected,
    }
}
fn adapter() -> Result<LinterOxlintAdapter, Failure> {
    Ok(LinterOxlintAdapter::new(LinterOxlintSettings {
        executable: "oxlint".into(),
        defaults:
            [RuleId::try_from("typescript.no-this-alias").map_err(|_blank| Refusal::Invalid)?]
                .into_iter()
                .collect(),
    }))
}
/// A request of the named files that runs the pinned defaults, keeping its state under `state`.
fn request(files: &[&str], state: &std::path::Path) -> RunRequest {
    RunRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots:
            ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata:
            ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: "root".into(),
        state: state.to_owned(),
        files: files
            .iter()
            .map(|file| RelativePath((*file).into()))
            .collect(),
    }
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
                .profile(ArtifactPath(std::path::Path::new(env!(
                    "CARGO_MANIFEST_DIR"
                ))))
                .map_err(|error| refusal(&error))?;
            Ok(adapter.version_line(&tool).0.into_string().into())
        }
        Call::Prepare(files) => {
            let temporary = tempfile::tempdir().map_err(|_error| Refusal::Unexpected)?;
            let state = temporary.path().join("state");
            let invocation = adapter
                .prepare(&request(files, &state))
                .map_err(|error| refusal(&error))?;
            Ok(format!(
                "{}: {}",
                invocation.directory.display(),
                invocation
                    .arguments
                    .iter()
                    .map(|argument| argument.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .replace(&format!("{}/", temporary.path().display()), "")
            .into())
        }
        Call::Configuration => {
            let temporary = tempfile::tempdir().map_err(|_error| Refusal::Unexpected)?;
            let invocation = adapter
                .prepare(&request(&["a.ts"], &temporary.path().join("state")))
                .map_err(|error| refusal(&error))?;
            let configuration = invocation
                .arguments
                .iter()
                .skip_while(|argument| *argument != "-c")
                .nth(1)
                .ok_or(Refusal::Unexpected)?;
            Ok(std::fs::read_to_string(configuration)
                .map_err(|_error| Refusal::Unexpected)?
                .into())
        }
        Call::Decode(status, stdout) => {
            let temporary = tempfile::tempdir().map_err(|_error| Refusal::Unexpected)?;
            let request = request(&["alias.ts"], &temporary.path().join("state"));
            let report = adapter
                .decode(
                    &request,
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
