//! Pure byte accounting for typed equality storage; acquisition lives in application.

use crate::domain::table_model::types::{ChainLink, HashedRow, KeyCellsHash, KeyIndex};
const ARC_BYTES: usize =
    size_of::<KeyIndex>() + 2 * size_of::<usize>() + align_of::<KeyIndex>() - 1;
/// Sum actual capacities of every index vector and the Arc allocation metadata.
/// Stored indexes never allocate the grouping-only hash set; refuse rather than undercharge it.
pub(crate) fn index_bytes(index: &KeyIndex) -> Option<usize> {
    if index.runs.capacity() != 0 {
        return None;
    }
    let vectors = [
        (index.rows.capacity(), size_of::<RowIndex>()),
        (index.hashes.capacity(), size_of::<KeyCellsHash>()),
        (index.heads.capacity(), size_of::<ChainLink>()),
        (index.next.capacity(), size_of::<ChainLink>()),
        (index.sorted.capacity(), size_of::<RowIndex>()),
        (index.pairs.capacity(), size_of::<HashedRow>()),
    ];
    vectors
        .into_iter()
        .try_fold(ARC_BYTES, |bytes, (capacity, width)| {
            bytes.checked_add(capacity.checked_mul(width)?)
        })
}

use ryo_values::domain::rows::types::RowIndex;
