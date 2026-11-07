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
#[path = "project_inputs_test.rs"]
mod tests;
