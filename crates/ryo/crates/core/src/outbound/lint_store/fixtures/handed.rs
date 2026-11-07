//! The test support of the `lint_store` port that reads a snapshot's handed rows back as they were kept.

use super::super::errors::LintStoreError;
use super::super::interface::ReadStore;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stores::types::{BucketsWanted, StoredHanded, StoredWords};

/// Explicitly resolve the read representation only in contract result oracles.
/// Production handover resolves a filtered row at a time instead of materializing this table.
pub fn materialized_handed(
    read: &dyn ReadStore,
    wanted: &BucketsWanted,
) -> Result<StoredHanded, LintStoreError> {
    let handed = read.handed(wanted)?.0;
    let mut rows = Vec::with_capacity(handed.frontier.len());
    for row in handed.frontier {
        let words = match row.payload {
            ryo_values::domain::stores::types::ReadWordPayload::Inline(words) => words,
            ryo_values::domain::stores::types::ReadWordPayload::Old(span) => {
                let mut loaded = PackedWords::default();
                read.read_old(&span, &mut loaded)?;
                loaded
            }
        };
        rows.push(StoredWords {
            file: row.file,
            bucket: row.bucket,
            proof: row.proof,
            words,
        });
    }
    Ok(StoredHanded(
        ryo_values::domain::stores::types::HandedRows {
            observations: handed.observations,
            proofs: handed.proofs,
            frontier: rows,
            texts: handed.texts,
            notes: handed.notes,
            covered: handed.covered,
        },
    ))
}
