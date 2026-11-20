//! The packs document of the postcard store, a section of its own: the compiled pack of each rule set the new engine ran, under
//! its key, with what a row of each rule it runs emits. A run of the old engine alone never reads the section, and a run of the
//! new engine reads nothing else of a pack.

use crate::outbound::lint_store::adapters::postcard::strings::schemas::as_bytes;
use ryo_relations::domain::relations::types::Bytes;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Heading, StepId};
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::stores::types::{PackKey, StoredPack};
use std::borrow::Cow;

/// One rule set's compiled pack: what it is kept under, its bytes as the engine wrote them, and what its rules emit.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct PackSchema<'a> {
    /// What the pack is kept under.
    pub key: [u8; 32],
    /// The pack's bytes.
    #[serde(borrow, serialize_with = "as_bytes")]
    pub bytes: Cow<'a, [u8]>,
    /// What a row of each rule the rule set runs emits, in the order of those rules.
    #[serde(borrow)]
    pub emits: Vec<EmitSchema<'a>>,
}

/// What a row of a rule emits.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct EmitSchema<'a> {
    /// The step whose rows answer the rule.
    pub step: u32,
    /// The column holding the file a row is about.
    pub subject: u32,
    /// The columns that identify a row.
    pub key: Vec<u32>,
    /// The headings of the answer's columns.
    #[serde(borrow)]
    pub headings: Vec<Cow<'a, str>>,
}

impl<'a> From<&'a Emit> for EmitSchema<'a> {
    fn from(emit: &'a Emit) -> Self {
        Self {
            step: emit.step.0,
            subject: emit.subject.0,
            key: emit.key.iter().map(|column| column.0).collect(),
            headings: emit
                .headings
                .iter()
                .map(|heading| Cow::Borrowed(&*heading.0))
                .collect(),
        }
    }
}

impl From<EmitSchema<'_>> for Emit {
    fn from(schema: EmitSchema<'_>) -> Self {
        Self {
            step: StepId(schema.step),
            subject: ColumnIndex(schema.subject),
            key: schema.key.into_iter().map(ColumnIndex).collect(),
            headings: schema
                .headings
                .into_iter()
                .map(|heading| Heading(heading.into()))
                .collect(),
        }
    }
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
            emits: pack.emits.iter().map(EmitSchema::from).collect(),
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
                    emits: pack.emits.into_iter().map(Emit::from).collect(),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
