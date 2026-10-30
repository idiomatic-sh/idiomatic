//! Initialization through the command edge uses an actual empty project.
use super::command::{InitArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::workspace::check::fixtures::edge;
type Input = bool;
type Value = bool;
type Failure = Outcome;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("initializes"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("preserves_existing_configuration"),
        word: Word::Refuses,
        input: || true,
        expect: Expect::error(Outcome::Error),
    },
];
fn subject(existing: Input) -> Result<Value, Failure> {
    let directory = tempfile::tempdir().expect("temporary project");
    // A project is a git repository: `init` refuses one outside any.
    std::fs::create_dir_all(directory.path().join(".git")).expect("the repository's marker");
    if existing {
        std::fs::write(directory.path().join("tastemaker.toml"), "user content")
            .expect("existing manifest");
    }
    let at = edge(&directory);
    let _answer = answer(InitArguments, &at)?;
    Ok(directory.path().join("tastemaker/package.toml").is_file())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
