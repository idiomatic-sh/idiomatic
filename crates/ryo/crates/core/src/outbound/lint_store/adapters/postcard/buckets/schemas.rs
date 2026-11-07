//! The buckets document of the postcard store: its wire shapes and their conversion from and into the core's tables.

use std::borrow::Cow;
#[cfg(test)]
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_findings::types::StoredFileNumber;
#[cfg(test)]
use ryo_values::domain::stores::types::StoredWords;
use super::super::strings::schemas::as_bytes;

/// The words one file handed one bucket: one row for each file, in a section of the bucket's own.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct WordsSchema<'a> {
    /// The file, by its place among the files.
    pub(in crate::outbound::lint_store::adapters::postcard) file: u32,
    /// The words, packed.
    #[serde(borrow, serialize_with = "as_bytes")]
    pub(in crate::outbound::lint_store::adapters::postcard) words: Cow<'a, [u8]>,
}

/// The words the files of `stored` handed `bucket`, as its section's bytes; `rows` are the rows of that bucket.
#[cfg(test)]
pub(in crate::outbound::lint_store::adapters::postcard) fn bucket_bytes(rows: &[&StoredWords]) -> Result<Vec<u8>, postcard::Error> {
    let words: Vec<WordsSchema<'_>> = rows
        .iter()
        .map(|each| WordsSchema {
            file: each.file.0,
            words: Cow::Borrowed(&each.words.0),
        })
        .collect();
    // The room the bytes take: each row's words and the two numbers before them.
    let room = rows
        .iter()
        .map(|each| each.words.0.len() + 10)
        .sum::<usize>()
        + 10;
    postcard::to_extend(&words, Vec::with_capacity(room))
}

/// The words of a bucket made again, as its section's bytes: those of the files read now, `fresh`, and, of the section `old` the
/// store kept of the bucket, those of each file used again, under its number now (`file_again`, old number to new, in the order of
/// both); in the order of the files. The words of `old` are copied as they are, never decoded. None when no file hands it any.
#[cfg(test)]
pub(in crate::outbound::lint_store::adapters::postcard) fn rebuilt_bucket_bytes(
    fresh: &[&StoredWords],
    old: Option<&[u8]>,
    file_again: &[ryo_values::domain::stores::types::FileAgain],
) -> Result<Option<Vec<u8>>, postcard::Error> {
    let mut words: Vec<WordsSchema<'_>> = fresh
        .iter()
        .map(|each| WordsSchema {
            file: each.file.0,
            words: Cow::Borrowed(&each.words.0),
        })
        .collect();
    let kept: Vec<WordsSchema<'_>> = match old {
        Some(bytes) => postcard::from_bytes(bytes)?,
        None => Vec::new(),
    };
    words.extend(kept.into_iter().filter_map(|each| {
        // A file not used again is gone, with its words.
        match file_again.binary_search_by_key(&each.file, |again| again.was.0) {
            Ok(place) => file_again.get(place).map(|again| WordsSchema {
                file: again.now.0,
                words: each.words,
            }),
            Err(_) => None,
        }
    }));
    if words.is_empty() {
        return Ok(None);
    }
    words.sort_by_key(|each| each.file);
    // The room the bytes take: each row's words and the two numbers before them.
    let room = words
        .iter()
        .map(|each| each.words.len() + 10)
        .sum::<usize>()
        + 10;
    postcard::to_extend(&words, Vec::with_capacity(room)).map(Some)
}

/// The words the files handed `bucket`, from its section's bytes.
#[cfg(test)]
pub(in crate::outbound::lint_store::adapters::postcard) fn bucket_from(
    bucket: Bucket,
    bytes: &[u8],
    proofs: &[ryo_values::domain::stores::types::StoredFrontierProof],
) -> Option<Vec<StoredWords>> {
    let Ok(rows) = postcard::from_bytes::<Vec<WordsSchema<'_>>>(bytes) else {
        return None;
    };
    rows.into_iter()
        .map(|each| {
            Some(StoredWords {
                file: StoredFileNumber(each.file),
                bucket,
                proof: proofs
                    .iter()
                    .find(|row| row.file.0 == each.file && row.bucket == bucket)?
                    .proof
                    .clone(),
                words: PackedWords(each.words.into_owned()),
            })
        })
        .collect::<Option<Vec<_>>>()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
