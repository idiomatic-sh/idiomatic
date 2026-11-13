//! Canonical identity for optional project namespace inputs used by both contestants.
use super::constants;
use crate::domain::canonical_text::{
    service::digest as document_digest,
    types::{Fragment, Label},
};
use ryo::linter::{
    GivenProjectInventory, ProjectEntryState, ProjectInventoryCompleteness, RelativePath,
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::constants::MISSING;
use std::collections::BTreeMap;

/// Explicit project context only; legacy undeclared/partial-empty cases retain no new digest.
fn document(roots: Option<&[RelativePath]>, inventory: &GivenProjectInventory) -> Option<Fragment> {
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
                ProjectEntryState::File => constants::ENTRY_FILE,
                ProjectEntryState::Directory => constants::ENTRY_DIRECTORY,
                ProjectEntryState::Other => constants::ENTRY_OTHER,
                ProjectEntryState::Missing => MISSING,
                ProjectEntryState::Unknown => constants::ENTRY_UNKNOWN,
            };
            Fragment::Object(BTreeMap::from([
                (
                    Label::from(constants::ENTRY_PATH),
                    Fragment::from(entry.path.0.as_ref()),
                ),
                (Label::from(constants::ENTRY_STATE), Fragment::from(state)),
            ]))
        })
        .collect::<Vec<_>>();
    let completeness = match inventory.completeness {
        ProjectInventoryCompleteness::Partial => constants::PARTIAL,
        ProjectInventoryCompleteness::CompleteWithinDeclaredScope => {
            constants::COMPLETE_WITHIN_DECLARED_SCOPE
        }
    };
    Some(Fragment::Object(BTreeMap::from([
        (
            Label::from(constants::PROJECT_SEARCH_ROOTS),
            roots.map_or(Fragment::Null, |roots| {
                Fragment::List(
                    roots
                        .iter()
                        .map(|root| Fragment::from(root.0.as_ref()))
                        .collect(),
                )
            }),
        ),
        (
            Label::from(constants::GIVEN_PROJECT_INVENTORY),
            Fragment::Object(BTreeMap::from([
                (
                    Label::from(constants::COMPLETENESS),
                    Fragment::from(completeness),
                ),
                (Label::from(constants::ENTRIES), Fragment::List(entries)),
            ])),
        ),
    ])))
}

/// Hash the canonical JSON bytes; object keys are sorted by the canonical document renderer.
#[must_use]
pub fn digest(
    roots: Option<&[RelativePath]>,
    inventory: &GivenProjectInventory,
) -> Option<ArtifactDigest> {
    document(roots, inventory).map(|fragment| document_digest(&fragment))
}
