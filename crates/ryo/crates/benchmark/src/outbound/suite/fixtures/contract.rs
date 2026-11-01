//! Missing implementations remain visible and an empty selection is not coverage.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::Tool};
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{ArtifactPath, ArtifactSet};
#[derive(Clone, Copy, Debug)]
/// Artifact operations exercised against missing inputs as well as valid inventories.
pub enum Operation {
    /// Audit an inventory with one unimplemented default.
    Audit,
    /// Refuse an empty default inventory.
    Empty,
    /// Read a missing profile.
    Profile,
    /// Read a missing shared workload.
    Projects,
    /// Verify a missing executable.
    Verify,
    /// Hash a missing artifact.
    Fingerprint,
    /// Hash one missing executable's exact bytes.
    Digest,
    /// Stage a missing rule inventory.
    Stage,
    /// Stage a missing project manifest.
    StageProjects,
    /// Materialize missing independent cases.
    Cases,
}
/// Artifact operation to exercise.
type Input = Operation;
/// Counts of authored and missing default rules.
type Value = (usize, usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Canonical contract failure classifications.
pub enum Refusal {
    /// The reference default inventory is empty.
    EmptyDefaults,
    /// The implementation returned an unexpected failure category.
    Unexpected,
    /// The requested artifact does not exist.
    MissingArtifact,
    /// Empty materialized rule sources cannot establish correctness.
    NoCases,
}
/// Named observations every implementation must satisfy.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("missing profile is refused"),
        word: Word::Refuses,
        input: || Operation::Profile,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing shared manifest is refused"),
        word: Word::Refuses,
        input: || Operation::Projects,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing executable is refused"),
        word: Word::Refuses,
        input: || Operation::Verify,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing fingerprint input is refused"),
        word: Word::Refuses,
        input: || Operation::Fingerprint,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing executable digest input is refused"),
        word: Word::Refuses,
        input: || Operation::Digest,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing rule staging input is refused"),
        word: Word::Refuses,
        input: || Operation::Stage,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing project staging input is refused"),
        word: Word::Refuses,
        input: || Operation::StageProjects,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("empty case sources are refused"),
        word: Word::Refuses,
        input: || Operation::Cases,
        expect: Expect::Error(Failure::NoCases),
    },
    Case {
        name: CaseName("missing implementation stays missing"),
        word: Word::Succeeds,
        input: || Operation::Audit,
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("empty inventory is refused"),
        word: Word::Refuses,
        input: || Operation::Empty,
        expect: Expect::Error(Failure::EmptyDefaults),
    },
];
/// Run the shared observations against the supplied implementation.
///
/// # Panics
///
/// Panics when fixture setup fails or the implementation violates a contract observation.
#[expect(
    clippy::expect_used,
    reason = "fixture setup failures must fail the contract test"
)]
pub fn check(adapter: &impl SuitePort) {
    let temporary = tempfile::tempdir().expect("suite fixture");
    std::fs::create_dir_all(temporary.path().join("rules")).expect("rules fixture");
    let subject = |operation| {
        let selected = !matches!(operation, Operation::Empty);
        let tool = Tool {
            project_context_mode: Default::default(),
            tool: "fixture".into(),
            version: "1".into(),
            sha256: vec![String::new().into()],
            deferred: std::collections::BTreeMap::default(),
            pending: std::collections::BTreeMap::default(),
            query_modules: std::collections::BTreeMap::default(),
            rule_applicability: std::collections::BTreeMap::default(),
            parser_configuration: None,
            defaults: if selected {
                vec!["fixture".try_into().expect("rule identifier")]
            } else {
                Vec::new()
            },
        };
        let missing = temporary.path().join("missing");
        let destination = temporary.path().join("destination");
        let result = match operation {
            Operation::Audit | Operation::Empty => adapter
                .audit(ArtifactPath(temporary.path()), &tool)
                .map(|audit| (audit.implemented.len(), audit.missing.len())),
            Operation::Profile => adapter.profile(ArtifactPath(&missing)).map(|_| (0, 0)),
            Operation::Projects => adapter.projects(ArtifactPath(&missing)).map(|_| (0, 0)),
            Operation::Verify => adapter
                .verify_tool(ArtifactPath(&missing), &tool)
                .map(|()| (0, 0)),
            Operation::Fingerprint => adapter.fingerprint(ArtifactSet(&[missing])).map(|_| (0, 0)),
            Operation::Stage => {
                let audit = adapter
                    .audit(ArtifactPath(temporary.path()), &tool)
                    .expect("valid inventory");
                adapter
                    .stage(ArtifactPath(&missing), ArtifactPath(&destination), &audit)
                    .map(|()| (0, 0))
            }
            Operation::Digest => adapter.digest(ArtifactPath(&missing)).map(|_| (0, 0)),
            Operation::StageProjects => adapter
                .stage_projects(ArtifactPath(&missing), ArtifactPath(&destination))
                .map(|()| (0, 0)),
            Operation::Cases => adapter
                .cases(
                    ArtifactPath(&temporary.path().join("rules")),
                    ArtifactPath(&destination),
                )
                .map(|_| (0, 0)),
        };
        result.map_err(|error| match error {
            Error::Invalid(message) if message == "empty or duplicated default rule inventory" => {
                Failure::EmptyDefaults
            }
            Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Failure::MissingArtifact
            }
            Error::Invalid(message) if message == "no executable rule cases" => Failure::NoCases,
            Error::Invalid(_) | Error::Io(_) | Error::Json(_) | Error::Toml(_) => {
                Failure::Unexpected
            }
        })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

/// Classified artifact validation refusal.
type Failure = Refusal;
