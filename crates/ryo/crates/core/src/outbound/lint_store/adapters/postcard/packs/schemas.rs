//! The packs document of the postcard store, a section of its own: the compiled pack of each rule set the new engine ran, under
//! its key. A run of the old engine alone never reads the section, and a run of the new engine reads nothing else of a pack.

use crate::outbound::lint_store::adapters::postcard::strings::schemas::as_bytes;
use ryo_relations::domain::relations::types::Bytes;
use ryo_values::domain::stores::types::{PackKey, StoredPack};
use std::borrow::Cow;

/// One rule set's compiled pack: what it is kept under, and its bytes as the engine wrote them.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct PackSchema<'a> {
    /// What the pack is kept under.
    pub key: [u8; 32],
    /// The pack's bytes.
    #[serde(borrow, serialize_with = "as_bytes")]
    pub bytes: Cow<'a, [u8]>,
}

/// The packs, as the bytes of their section.
pub(in crate::outbound::lint_store::adapters::postcard) fn packs_bytes(
    packs: &[StoredPack],
) -> Result<Vec<u8>, postcard::Error> {
    let schemas: Vec<PackSchema<'_>> = packs
        .iter()
        .map(|pack| PackSchema {
            key: pack.key.0,
            bytes: Cow::Borrowed(&pack.bytes.0),
        })
        .collect();
    postcard::to_stdvec(&schemas)
}

/// The packs the section's bytes hold; none when they do not read back as packs.
pub(in crate::outbound::lint_store::adapters::postcard) fn packs_of(
    bytes: &[u8],
) -> Vec<StoredPack> {
    postcard::from_bytes::<Vec<PackSchema<'_>>>(bytes)
        .map(|schemas| {
            schemas
                .into_iter()
                .map(|pack| StoredPack {
                    key: PackKey(pack.key),
                    bytes: Bytes(pack.bytes.into_owned()),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
