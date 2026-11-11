//! Paths that leave their package: an external crate names nothing of the repository, an external target with parent segments, and a path into a library the package depends on.
use super::{Repository, declares, facts, package, writes};
use crate::cases::{Case, CaseName, Expect, Word, run};

fn a_path_to_an_external_crate_names_nothing_of_the_repository() -> Repository {
    package(
        vec!["p/Cargo.toml", "p/src/lib.rs"],
        vec![(
            "p/src/lib.rs",
            facts(
                Vec::new(),
                vec![writes(&["std", "mem", "swap"]), writes(&["Self", "new"])],
            ),
        )],
    )
}

/// Cargo retains parent segments in a target path; source discovery normalizes them.
fn an_external_target_with_parent_segments() -> Repository {
    Repository {
        files: vec![
            "distribution/child.rs",
            "distribution/main.rs",
            "p/Cargo.toml",
        ],
        roots: vec![("p", "p/../distribution/main.rs")],
        outputs: Vec::new(),
        facts: vec![
            (
                "distribution/main.rs",
                facts(declares(&["child"]), vec![writes(&["child"])]),
            ),
            ("distribution/child.rs", facts(Vec::new(), Vec::new())),
        ],
        externs: Vec::new(),
    }
}

/// Two crates: `a` depends on `b` and writes `bee` for it. Its path resolves from `b`'s crate root; `b` depends on nothing, so the
/// same path written in `b` names nothing.
fn a_path_into_a_library_the_package_depends_on() -> Repository {
    let path = || writes(&["bee", "inner", "f"]);
    Repository {
        files: vec![
            "a/Cargo.toml",
            "a/src/lib.rs",
            "b/Cargo.toml",
            "b/src/inner.rs",
            "b/src/lib.rs",
        ],
        roots: vec![("a", "a/src/lib.rs"), ("b", "b/src/lib.rs")],
        outputs: Vec::new(),
        facts: vec![
            ("a/src/lib.rs", facts(Vec::new(), vec![path()])),
            ("b/src/inner.rs", facts(Vec::new(), Vec::new())),
            ("b/src/lib.rs", facts(declares(&["inner"]), vec![path()])),
        ],
        externs: vec![("a", "bee", "b", "b/src/lib.rs")],
    }
}

/// What a row is given: the parent table's input.
type Input = super::Input;
/// What a row expects: the parent table's value.
type Value = super::Value;
/// What a refused row expects: the parent table's failure.
type Failure = super::Failure;

/// Runs one row through the parent table's subject.
fn subject(input: Input) -> Value {
    super::subject(input)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_path_to_an_external_crate_names_nothing_of_the_repository"),
        word: Word::Succeeds,
        input: || a_path_to_an_external_crate_names_nothing_of_the_repository,
        expect: Expect::Value(" | p/src/lib.rs"),
    },
    Case {
        name: CaseName("a_declared_external_target_and_its_modules_are_analyzed"),
        word: Word::Succeeds,
        input: || an_external_target_with_parent_segments,
        expect: Expect::Value(
            "distribution/main.rs -> distribution/child.rs | distribution/child.rs distribution/main.rs",
        ),
    },
    Case {
        name: CaseName("a_path_into_a_library_the_package_depends_on_names_the_file_in_that_crate"),
        word: Word::Succeeds,
        input: || a_path_into_a_library_the_package_depends_on,
        expect: Expect::Value(
            "a/src/lib.rs -> b/src/inner.rs | a/src/lib.rs b/src/inner.rs b/src/lib.rs",
        ),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
