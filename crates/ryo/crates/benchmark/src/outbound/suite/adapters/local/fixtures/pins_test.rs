//! Actual executable bytes and version refusals prove the pin boundary.
use super::super::adapter::SuiteLocalAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::types::ArtifactPath;
#[cfg(unix)]
const CASES: &[Case<(&str, &str, bool), (), &str>] = &[
    Case {
        name: CaseName("matching executable bytes and version are accepted"),
        word: Word::Succeeds,
        input: || {
            (
                "b145a69e05e755a39d063badddeffe112a67d4f55993e373cc4db2074b984c0b",
                "1.0",
                true,
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("wrong digest refuses before attempting a non executable file"),
        word: Word::Refuses,
        input: || ("wrong", "1.0", false),
        expect: Expect::Error("hash"),
    },
    Case {
        name: CaseName("matching bytes do not bypass the expected version"),
        word: Word::Refuses,
        input: || {
            (
                "b145a69e05e755a39d063badddeffe112a67d4f55993e373cc4db2074b984c0b",
                "2.0",
                true,
            )
        },
        expect: Expect::Error("version"),
    },
];

#[cfg(unix)]
fn verify_pin((sha256, version, executable): (&str, &str, bool)) -> Result<(), &'static str> {
    use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::Tool};
    use crate::outbound::suite::interface::SuitePort;
    let directory = tempfile::tempdir().expect("tool directory");
    let path = directory.path().join("fixture");
    std::fs::write(&path, b"#!/bin/sh\necho fixture 1.0\n").expect("fixture bytes");
    permissions(&path, executable).expect("fixture mode");
    let tool = Tool {
        tool: "fixture".into(),
        version: version.into(),
        sha256: sha256.into(),
        deferred: Default::default(),
        pending: Default::default(),
        defaults: vec!["E722".try_into().expect("rule identifier")],
    };
    SuiteLocalAdapter
        .verify_tool(ArtifactPath(&path), &tool)
        .map_err(|error| match error {
            Error::Invalid(message) if message == "tool executable hash differs from pin" => "hash",
            Error::Invalid(message) if message == "tool version differs from pin" => "version",
            _ => "unexpected error",
        })
}

#[cfg(unix)]
fn permissions(path: &std::path::Path, executable: bool) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
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
