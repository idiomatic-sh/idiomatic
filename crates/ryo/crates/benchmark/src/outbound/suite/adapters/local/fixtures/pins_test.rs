//! Actual executable bytes and version refusals prove the pin boundary.
use super::super::adapter::SuiteLocalAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::types::ArtifactPath;
#[cfg(unix)]
type PinCase = Case<(&'static [&'static str], &'static str, PinFixture), (), Failure>;
/// Immutable executables differ in bytes; refused non-executables are independently provisioned.
#[derive(Clone, Copy)]
enum PinFixture {
    /// Original approved binary.
    Original,
    /// Distinct approved executable with identical version output.
    Alternate,
    /// Bytes cannot execute, proving hash refusal happens first.
    NonExecutable,
}
#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Hash,
    Version,
    Unexpected(String),
}
#[cfg(unix)]
const CASES: &[PinCase] = &[
    Case {
        name: CaseName("matching executable bytes and version are accepted"),
        word: Word::Succeeds,
        input: || {
            (
                &["b145a69e05e755a39d063badddeffe112a67d4f55993e373cc4db2074b984c0b"],
                "1.0",
                PinFixture::Original,
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an independently pinned alternate executable is accepted"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "b145a69e05e755a39d063badddeffe112a67d4f55993e373cc4db2074b984c0b",
                    "77969ba69caaf5db2ee1bbea84cc3f917a3f21afaa4e964e89f1ab9671074de0",
                ],
                "1.0",
                PinFixture::Alternate,
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an empty pin set refuses before executing"),
        word: Word::Refuses,
        input: || (&[], "1.0", PinFixture::NonExecutable),
        expect: Expect::Error(Failure::Hash),
    },
    Case {
        name: CaseName("wrong digest refuses before attempting a non executable file"),
        word: Word::Refuses,
        input: || {
            (
                &["wrong", "another-wrong-pin"],
                "1.0",
                PinFixture::NonExecutable,
            )
        },
        expect: Expect::Error(Failure::Hash),
    },
    Case {
        name: CaseName("matching bytes do not bypass the expected version"),
        word: Word::Refuses,
        input: || {
            (
                &["b145a69e05e755a39d063badddeffe112a67d4f55993e373cc4db2074b984c0b"],
                "2.0",
                PinFixture::Original,
            )
        },
        expect: Expect::Error(Failure::Version),
    },
];

#[cfg(unix)]
#[expect(
    clippy::expect_used,
    reason = "fixed executable pin fixtures must be created before testing verification"
)]
fn verify_pin((sha256, version, fixture): (&[&str], &str, PinFixture)) -> Result<(), Failure> {
    use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::Tool};
    use crate::outbound::suite::interface::SuitePort as _;
    let directory = tempfile::tempdir().expect("tool directory");
    // Parallel process tests can fork while a temporary executable is still open for writing.
    // Execute immutable repository bytes; only non-executable refusal cases need a new file.
    let path = match fixture {
        PinFixture::Original => std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/outbound/suite/adapters/local/fixtures/pins.sh"),
        PinFixture::Alternate => std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/outbound/suite/adapters/local/fixtures/pins-alternate.sh"),
        PinFixture::NonExecutable => {
            let path = directory.path().join("fixture");
            std::fs::write(&path, include_bytes!("pins.sh")).expect("non-executable fixture bytes");
            permissions(&path, false).expect("fixture mode");
            path
        }
    };
    let tool = Tool {
        file_observation: None,
        project_context_mode: crate::domain::suite::types::ProjectContextMode::default(),
        tool: "fixture".into(),
        version: version.into(),
        sha256: sha256.iter().map(|value| (*value).into()).collect(),
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        defaults: vec!["E722".try_into().expect("rule identifier")],
    };
    SuiteLocalAdapter
        .verify_tool(ArtifactPath(&path), &tool)
        .map_err(|error| match error {
            Error::Invalid(message) if message == "tool executable hash differs from pin" => {
                Failure::Hash
            }
            Error::Invalid(message) if message == "tool version differs from pin" => {
                Failure::Version
            }
            Error::Invalid(_) | Error::Io(_) | Error::Json(_) | Error::Toml(_) => {
                Failure::Unexpected(error.to_string())
            }
        })
}

#[cfg(unix)]
fn permissions(path: &std::path::Path, executable: bool) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode(if executable { 0o700 } else { 0o600 }),
    )
}
#[cfg(not(unix))]
fn permissions(_: &std::path::Path, _: bool) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, verify_pin);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, verify_pin);
}
