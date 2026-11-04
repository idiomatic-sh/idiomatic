//! Canonical identity for optional project namespace inputs used by both contestants.
use super::types::ArtifactDigest;
use ryo::linter::{
    GivenProjectInventory, ProjectEntryState, ProjectInventoryCompleteness, RelativePath,
};
use sha2::{Digest as _, Sha256};

/// Explicit project context only; legacy undeclared/partial-empty cases retain no new digest.
#[must_use]
pub fn value(
    roots: Option<&[RelativePath]>,
    inventory: &GivenProjectInventory,
) -> Option<serde_json::Value> {
    if roots.is_none()
        && inventory.entries.is_empty()
        && inventory.completeness == ProjectInventoryCompleteness::Partial
    {
        return None;
    }
    let entries = inventory
        .entries
        .iter()
        .map(|entry| {
            let state = match entry.state {
                ProjectEntryState::File => "file",
                ProjectEntryState::Directory => "directory",
                ProjectEntryState::Other => "other",
                ProjectEntryState::Missing => "missing",
                ProjectEntryState::Unknown => "unknown",
            };
            serde_json::json!({"path": entry.path.0.as_ref(), "state": state})
        })
        .collect::<Vec<_>>();
    Some(serde_json::json!({
        "project_search_roots": roots.map(|roots| roots.iter().map(|root| root.0.as_ref()).collect::<Vec<_>>()),
        "given_project_inventory": {
            "completeness": match inventory.completeness {
                ProjectInventoryCompleteness::Partial => "partial",
                ProjectInventoryCompleteness::CompleteWithinDeclaredScope => "complete-within-declared-scope",
            },
            "entries": entries,
        }
    }))
}

/// Hash the canonical JSON bytes; object keys are sorted by serde_json's map representation.
pub fn digest(
    roots: Option<&[RelativePath]>,
    inventory: &GivenProjectInventory,
) -> Result<Option<ArtifactDigest>, serde_json::Error> {
    value(roots, inventory)
        .map(|value| {
            serde_json::to_vec(&value)
                .map(|bytes| ArtifactDigest(format!("{:x}", Sha256::digest(bytes))))
        })
        .transpose()
}

#[cfg(test)]
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
