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
                payload: ReadWordPayload::Inline(row.words),
            })
            .collect(),
        texts: stored.texts,
        notes: stored.notes,
        covered: stored.covered,
        clean: stored.clean,
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

/// Spike (ryo-fmx3, host3): the clean files of the prior store kept as `carry_proofs` keeps proofs: a bucket copied unchanged
/// keeps the clean files no fresh row names, and a file used again keeps its clean buckets under its new number.
pub fn carry_clean(current: &mut StoredHanded, old: &StoredHanded, kept: &KeptRows) {
    let made: Vec<Bucket> = current.proofs.iter().map(|row| row.bucket).collect();
    for each in &old.clean {
        let unchanged = kept.unchanged.binary_search(&each.bucket).is_ok() && !made.contains(&each.bucket);
        let files: Vec<crate::domain::stored_findings::types::StoredFileNumber> = each
            .files
            .iter()
            .filter_map(|file| {
                if unchanged {
                    return Some(*file);
                }
                kept.file_again
                    .binary_search_by_key(file, |again| again.was)
                    .ok()
                    .and_then(|place| kept.file_again.get(place))
                    .map(|again| again.now)
            })
            .collect();
        for file in files {
            clean_file(current, each.bucket, &each.frame, file);
        }
    }
    for each in &mut current.clean {
        each.files.sort_unstable();
        each.files.dedup();
    }
}

/// Spike (ryo-fmx3, host3): adds `file` to the clean files of `bucket`, whose clean frame is `frame`; false when the bucket's
/// clean frame is another, so the caller keeps the file's frame as a row of its own.
pub fn clean_file(
    stored: &mut StoredHanded,
    bucket: Bucket,
    frame: &crate::domain::records::types::CleanFrame,
    file: crate::domain::stored_findings::types::StoredFileNumber,
) -> bool {
    match stored.clean.binary_search_by_key(&bucket, |each| each.bucket) {
        Ok(place) => match stored.clean.get_mut(place) {
            Some(each) if each.frame == *frame => {
                each.files.push(file);
                true
            }
            _ => false,
        },
        Err(place) => {
            stored.clean.insert(
                place,
                super::types::StoredClean {
                    bucket,
                    frame: frame.clone(),
                    files: vec![file],
                },
            );
            true
        }
    }
}
