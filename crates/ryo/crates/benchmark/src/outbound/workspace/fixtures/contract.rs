//! Pinned workspace preparation and edit lifecycle are shared across adapters.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{diagnostics::errors::Error, suite::types::Project};
use crate::outbound::workspace::interface::WorkspacePort;
use std::{fs, path::Path, process::Command};
type Input = bool;
type Value = usize;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Canonical contract failure classifications.
pub enum Refusal {
    /// The requested revision is not an immutable commit pin.
    InvalidPin,
    /// The implementation returned an unexpected failure category.
    Unexpected,
}
/// Named observations every implementation must satisfy.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("commit pins select tracked input"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("mutable revision is refused"),
        word: Word::Refuses,
        input: || false,
        expect: Expect::Error(Failure::InvalidPin),
    },
];
fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("fixture git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf8")
        .trim()
        .into()
}
/// Run the shared observations against the supplied implementation.
///
/// # Panics
///
/// Panics when fixture setup fails or the implementation violates a contract observation.
pub fn check(adapter: &impl WorkspacePort) {
    let temporary = tempfile::tempdir().expect("fixture directory");
    let source = temporary.path().join("source");
    fs::create_dir(&source).expect("fixture source");
    drop(git(&source, &["init", "--quiet"]));
    fs::write(source.join("input.txt"), "before\n").expect("fixture file");
    drop(git(&source, &["add", "input.txt"]));
    drop(git(
        &source,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    ));
    let pin = git(&source, &["rev-parse", "HEAD"]);
    let cache = temporary.path().join("cache");
    let subject = |valid| {
        let project = Project {
            name: "fixture".into(),
            repository: source.to_string_lossy().into(),
            revision: if valid { pin.clone() } else { "main".into() },
            patch: "edit.patch".into(),
        };
        adapter
            .prepare(&project, &cache)
            .and_then(|workspace| adapter.tracked_files(&workspace))
            .map(|files| files.len())
            .map_err(|error| match error {
                Error::Invalid(_) => Failure::InvalidPin,
                _ => Failure::Unexpected,
            })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

type Failure = Refusal;
