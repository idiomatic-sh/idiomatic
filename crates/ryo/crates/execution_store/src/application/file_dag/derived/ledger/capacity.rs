//! Layout arithmetic for retained allocations. The ledger is the one owner of byte accounting,
//! so every estimate of reserved room is written here and nowhere else.
use super::types::{ByteCount, SlotCount};

#[cfg(test)]
mod test;

/// Account actual capacity rather than live length.
pub(crate) fn vector<T>(values: &Vec<T>) -> ByteCount {
    saturating::<T>(SlotCount(values.capacity()))
}

/// Bytes of `capacity` elements of `T`, or none when they overflow.
pub(crate) fn checked<T>(capacity: SlotCount) -> Option<ByteCount> {
    capacity.0.checked_mul(size_of::<T>()).map(ByteCount)
}

/// Bytes of `capacity` elements of `T`, saturating.
pub(crate) fn saturating<T>(capacity: SlotCount) -> ByteCount {
    ByteCount(capacity.0.saturating_mul(size_of::<T>()))
}

/// Bytes of a hash table of `capacity` slots holding `T`, with one control byte per slot.
pub(crate) fn table<T>(capacity: SlotCount) -> ByteCount {
    ByteCount(capacity.0.saturating_mul(size_of::<T>() + 1))
}

/// A header value followed by `capacity` elements of `T`, or none when they overflow.
pub(crate) fn headed<H, T>(capacity: SlotCount) -> Option<ByteCount> {
    size_of::<H>()
        .checked_add(checked::<T>(capacity)?.0)
        .map(ByteCount)
}

/// Arc allocation header and payload alignment, excluding separately owned vectors.
pub(crate) const fn arc_bytes<T>() -> ByteCount {
    ByteCount(size_of::<T>() + 2 * size_of::<usize>() + align_of::<T>() - 1)
}

/// A compact table's own metadata: its header, its frames, kinds and row offsets by reserved room.
pub(crate) fn compact_table(
    table: &ryo_values::domain::table_words::types::CompactTable,
) -> ByteCount {
    ByteCount(
        size_of::<ryo_values::domain::table_words::types::CompactTable>()
            + 2 * size_of::<usize>()
            + vector(&table.frames).0
            + vector(&table.kinds).0
            + vector(&table.row_offsets).0,
    )
}
