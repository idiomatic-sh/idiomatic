//! The digest of the project scope is stable across languages.
use super::service::digest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::{
    GivenProjectInventory, ProjectEntryState, ProjectInventoryCompleteness, RelativePath,
};

type Input = (Option<Vec<RelativePath>>, GivenProjectInventory);
type Value = Digest;
type Failure = ();

const PINNED: &str = "ff9b78535b775babaadae518c198876aa1c27ef21d0abb75e04588c25638df7d";

/// What a scope's digest is, against the pinned digest of the ordered full scope.
#[derive(Debug, PartialEq, Eq)]
enum Digest {
    /// The digest pinned for the ordered full scope.
    Pinned,
    /// No digest: a legacy case declares no scope.
    Absent,
    /// A digest that differs from the pinned one.
    Changed,
}

fn roots(count: usize) -> Option<Vec<RelativePath>> {
    Some(
        [RelativePath("src".into()), RelativePath("vendor".into())]
            .into_iter()
            .take(count)
            .collect(),
    )
}

fn inventory() -> GivenProjectInventory {
    GivenProjectInventory {
        entries: vec![
            ryo::linter::ProjectEntryObservation {
                path: RelativePath("src/pkg".into()),
                state: ProjectEntryState::Directory,
            },
            ryo::linter::ProjectEntryObservation {
                path: RelativePath("vendor/pkg.py".into()),
                state: ProjectEntryState::Missing,
            },
        ],
        completeness: ProjectInventoryCompleteness::CompleteWithinDeclaredScope,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordered scope and entry kinds have the stable cross-language digest"),
        word: Word::Succeeds,
        input: || (roots(2), inventory()),
        expect: Expect::Value(Digest::Pinned),
    },
    Case {
        name: CaseName("a legacy case without a declared scope has no digest"),
        word: Word::Succeeds,
        input: || (None, GivenProjectInventory::default()),
        expect: Expect::Value(Digest::Absent),
    },
    Case {
        name: CaseName("a changed root authority changes the digest"),
        word: Word::Succeeds,
        input: || (roots(1), inventory()),
        expect: Expect::Value(Digest::Changed),
    },
];

fn subject((roots, inventory): Input) -> Result<Value, Failure> {
    let digested = digest(roots.as_deref(), &inventory);
    Ok(match digested.map(|value| value.0) {
        None => Digest::Absent,
        Some(value) if value == PINNED => Digest::Pinned,
        Some(_) => Digest::Changed,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
