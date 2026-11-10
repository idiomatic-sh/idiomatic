//! What the execute bit observation reads from the filesystem, one row of `CASES` for each input.
use super::observed;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = u32;
type Value = bool;
type Failure = ();

/// A file reports an execute bit exactly when any of its execute permissions is set.
#[cfg(unix)]
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a file without execute permission reports no execute bit"),
        word: Word::Succeeds,
        input: || 0o644,
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a file with an execute permission reports the execute bit"),
        word: Word::Succeeds,
        input: || 0o744,
        expect: Expect::Value(true),
    },
];
/// Platforms without POSIX modes observe nothing.
#[cfg(not(unix))]
const CASES: &[Case<Input, Value, Failure>] = &[];

/// Writes one file with the given mode and observes it.
#[cfg(unix)]
fn subject(mode: Input) -> Result<Value, Failure> {
    use std::os::unix::fs::PermissionsExt as _;
    let root = tempfile::tempdir().map_err(|_unwritable| ())?;
    let file = root.path().join("case.py");
    std::fs::write(&file, b"x").map_err(|_unwritable| ())?;
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode))
        .map_err(|_unwritable| ())?;
    observed(&file).map_err(|_refused| ())
}
/// Platforms without POSIX modes refuse the observation.
#[cfg(not(unix))]
fn subject(_mode: Input) -> Result<Value, Failure> {
    observed(std::path::Path::new(".")).map_err(|_refused| ())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
