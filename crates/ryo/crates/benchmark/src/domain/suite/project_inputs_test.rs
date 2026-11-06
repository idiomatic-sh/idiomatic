//! The digest of the project scope is stable across languages.
use super::{
    GivenProjectInventory, ProjectEntryState, ProjectInventoryCompleteness, RelativePath, digest,
};

#[test]
fn ordered_scope_and_entry_kinds_have_a_stable_cross_language_digest() {
    let roots = [RelativePath("src".into()), RelativePath("vendor".into())];
    let inventory = GivenProjectInventory {
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
    };
    assert_eq!(
        digest(Some(&roots), &inventory)
            .expect("digest")
            .map(|value| value.0),
        Some("ff9b78535b775babaadae518c198876aa1c27ef21d0abb75e04588c25638df7d".into())
    );
    assert_eq!(
        digest(None, &GivenProjectInventory::default()).expect("legacy"),
        None
    );
    assert_ne!(
        digest(Some(&roots), &inventory).expect("original"),
        digest(Some(&roots[..1]), &inventory).expect("changed root authority")
    );
}
