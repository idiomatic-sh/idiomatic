//! Move inline handover values into the snapshot read representation, without IO.

use super::types::{HandedRows, KeptRows, ReadHanded, ReadWordPayload, ReadWords, StoredHanded};
use crate::domain::stored_facts::types::Bucket;

/// Custom and memory stores retain inline payloads while sharing all rest-table types.
#[must_use]
pub fn read_handed(stored: StoredHanded) -> ReadHanded {
    let stored = stored.0;
    ReadHanded(HandedRows {
        observations: stored.observations,
        proofs: stored.proofs,
        frontier: stored
            .frontier
            .into_iter()
            .map(|row| ReadWords {
                file: row.file,
                bucket: row.bucket,
                proof: row.proof,
                words: ReadWordPayload::Inline(row.words),
            })
            .collect(),
        texts: stored.texts,
        notes: stored.notes,
        covered: stored.covered,
    })
}

/// Keep metadata for exactly the payload buckets copied from the prior store.
/// Current fresh metadata already covers inline and staged rows alike.
pub fn carry_proofs(current: &mut StoredHanded, old: &StoredHanded, kept: &KeptRows) {
    let made: Vec<Bucket> = current.proofs.iter().map(|row| row.bucket).collect();
    current.proofs.extend(old.proofs.iter().filter_map(|row| {
        if kept.unchanged.binary_search(&row.bucket).is_ok() {
            return (!made.contains(&row.bucket)).then(|| row.clone());
        }
        match kept
            .file_again
            .binary_search_by_key(&row.file, |again| again.was)
        {
            Ok(place) => kept.file_again.get(place),
            Err(_) => None,
        }
        .map(|again| super::types::StoredFrontierProof {
            file: again.now,
            ..row.clone()
        })
    }));
    current.proofs.sort_by_key(|row| (row.file.0, row.bucket.0));
}
