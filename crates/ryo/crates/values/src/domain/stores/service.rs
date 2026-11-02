//! Move inline handover values into the snapshot read representation, without IO.

use super::types::{ReadHanded, ReadWordPayload, ReadWords, StoredHanded};

/// Custom and memory stores retain inline payloads while sharing all rest-table types.
#[must_use]
pub fn read_handed(stored: StoredHanded) -> ReadHanded {
    ReadHanded {
        frontier: stored
            .frontier
            .into_iter()
            .map(|row| ReadWords {
                file: row.file,
                bucket: row.bucket,
                words: ReadWordPayload::Inline(row.words),
            })
            .collect(),
        texts: stored.texts,
        notes: stored.notes,
        covered: stored.covered,
    }
}
