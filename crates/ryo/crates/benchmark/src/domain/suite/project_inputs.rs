//! Canonical identity for optional project namespace inputs used by both contestants.
use crate::domain::suite::constants;
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
                ProjectEntryState::File => constants::FILE,
                ProjectEntryState::Directory => constants::DIRECTORY,
                ProjectEntryState::Other => constants::OTHER,
                ProjectEntryState::Missing => constants::MISSING,
                ProjectEntryState::Unknown => constants::UNKNOWN,
            };
            serde_json::json!({constants::PATH: entry.path.0.as_ref(), constants::STATE: state})
        })
        .collect::<Vec<_>>();
    Some(serde_json::json!({
        constants::PROJECT_SEARCH_ROOTS: roots.map(|roots| roots.iter().map(|root| root.0.as_ref()).collect::<Vec<_>>()),
        constants::GIVEN_PROJECT_INVENTORY: {
            constants::COMPLETENESS: match inventory.completeness {
                ProjectInventoryCompleteness::Partial => constants::PARTIAL,
                ProjectInventoryCompleteness::CompleteWithinDeclaredScope => constants::COMPLETE_WITHIN_DECLARED_SCOPE,
            },
            constants::ENTRIES: entries,
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
    let roots = [
        RelativePath(constants::SRC.into()),
        RelativePath(constants::VENDOR.into()),
    ];
    let inventory = GivenProjectInventory {
        entries: vec![
            ryo::linter::ProjectEntryObservation {
                path: RelativePath(constants::SRC_PKG.into()),
                state: ProjectEntryState::Directory,
            },
            ryo::linter::ProjectEntryObservation {
                path: RelativePath(constants::VENDOR_PKG_PY.into()),
                state: ProjectEntryState::Missing,
            },
        ],
        completeness: ProjectInventoryCompleteness::CompleteWithinDeclaredScope,
    };
    assert_eq!(
        digest(Some(&roots), &inventory)
            .expect(constants::DIGEST)
            .map(|value| value.0),
        Some(constants::FF9B78535B775BABAADAE518C198876AA1C27EF21D0A.into())
    );
    assert_eq!(
        digest(None, &GivenProjectInventory::default()).expect(constants::LEGACY),
        None
    );
    assert_ne!(
        digest(Some(&roots), &inventory).expect(constants::ORIGINAL),
        digest(Some(&roots[..1]), &inventory).expect(constants::CHANGED_ROOT_AUTHORITY)
    );
}
