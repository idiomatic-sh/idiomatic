//! Canonical identity for optional project namespace inputs used by both contestants.
use ryo::linter::{
    GivenProjectInventory, ProjectEntryState, ProjectInventoryCompleteness, RelativePath,
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
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
                ProjectEntryState::File => crate::domain::suite::constants::FILE,
                ProjectEntryState::Directory => crate::domain::suite::constants::DIRECTORY,
                ProjectEntryState::Other => crate::domain::suite::constants::OTHER,
                ProjectEntryState::Missing => crate::domain::suite::constants::MISSING,
                ProjectEntryState::Unknown => crate::domain::suite::constants::UNKNOWN,
            };
            serde_json::json!({crate::domain::suite::constants::PATH: entry.path.0.as_ref(), crate::domain::suite::constants::STATE: state})
        })
        .collect::<Vec<_>>();
    Some(serde_json::json!({
        crate::domain::suite::constants::PROJECT_SEARCH_ROOTS: roots.map(|roots| roots.iter().map(|root| root.0.as_ref()).collect::<Vec<_>>()),
        crate::domain::suite::constants::GIVEN_PROJECT_INVENTORY: {
            crate::domain::suite::constants::COMPLETENESS: match inventory.completeness {
                ProjectInventoryCompleteness::Partial => crate::domain::suite::constants::PARTIAL,
                ProjectInventoryCompleteness::CompleteWithinDeclaredScope => crate::domain::suite::constants::COMPLETE_WITHIN_DECLARED_SCOPE,
            },
            crate::domain::suite::constants::ENTRIES: entries,
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
    let roots = [RelativePath(crate::domain::suite::constants::SRC.into()), RelativePath(crate::domain::suite::constants::VENDOR.into())];
    let inventory = GivenProjectInventory {
        entries: vec![
            ryo::linter::ProjectEntryObservation {
                path: RelativePath(crate::domain::suite::constants::SRC_PKG.into()),
                state: ProjectEntryState::Directory,
            },
            ryo::linter::ProjectEntryObservation {
                path: RelativePath(crate::domain::suite::constants::VENDOR_PKG_PY.into()),
                state: ProjectEntryState::Missing,
            },
        ],
        completeness: ProjectInventoryCompleteness::CompleteWithinDeclaredScope,
    };
    assert_eq!(
        digest(Some(&roots), &inventory)
            .expect(crate::domain::suite::constants::DIGEST)
            .map(|value| value.0),
        Some(crate::domain::suite::constants::FF9B78535B775BABAADAE518C198876AA1C27EF21D0A.into())
    );
    assert_eq!(
        digest(None, &GivenProjectInventory::default()).expect(crate::domain::suite::constants::LEGACY),
        None
    );
    assert_ne!(
        digest(Some(&roots), &inventory).expect(crate::domain::suite::constants::ORIGINAL),
        digest(Some(&roots[..1]), &inventory).expect(crate::domain::suite::constants::CHANGED_ROOT_AUTHORITY)
    );
}
