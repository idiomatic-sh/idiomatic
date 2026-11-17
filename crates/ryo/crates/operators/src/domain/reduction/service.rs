//! Count/digest semantics over borrowed engine-owned populations, indexes, and reduction scratch.
use crate::domain::reduction::types::{DigestDatum, GroupDigest, HashScratch, ValueHash};
use ryo_values::domain::{plan_parts::types::FlagState, rows::types::RowCount};
use std::hash::Hasher as _;

/// The digest of a group from the hashes of its values: the distinct hashes, sorted, folded in order, and how many there
/// are. The one fold of a group digest, the row meaning's and the `group` kernel's.
pub(crate) fn folded(scratch: HashScratch<'_>) -> crate::domain::reduction::types::FoldedDigest {
    let hashes = scratch.0;
    hashes.sort_unstable();
    hashes.dedup();
    let mut folded = rustc_hash::FxHasher::default();
    for hash in hashes.iter() {
        folded.write_u64(*hash);
    }
    crate::domain::reduction::types::FoldedDigest {
        counted: RowCount(hashes.len()),
        digest: GroupDigest(folded.finish().cast_signed()),
    }
}

pub(crate) fn hash_of(datum: DigestDatum<'_>) -> ValueHash {
    let mut hasher = rustc_hash::FxHasher::default();
    match datum {
        DigestDatum::Unlocated => {
            hasher.write(ryo_relations::domain::relations::constants::UNLOCATED.as_bytes())
        }
        DigestDatum::Text(text) => hasher.write(text.0.as_bytes()),
        DigestDatum::Number(number) => hasher.write_i64(number.0),
        DigestDatum::Flag(flag) => hasher.write_u8(u8::from(flag == FlagState::Raised)),
        DigestDatum::Numbers(numbers) => {
            numbers.iter().for_each(|number| hasher.write_i64(number.0))
        }
        DigestDatum::Texts(items) => items
            .iter()
            .for_each(|text| hasher.write(text.0.as_bytes())),
    }
    ValueHash(hasher.finish())
}
