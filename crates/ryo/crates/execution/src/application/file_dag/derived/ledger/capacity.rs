//! Layout arithmetic for retained allocations. The ledger is the one owner of byte accounting,
//! so every estimate of reserved room is written here and nowhere else.

#[cfg(test)]
mod test;

/// Account actual capacity rather than live length.
pub(in crate::application::file_dag) fn vector<T>(values: &Vec<T>) -> usize {
    values.capacity().saturating_mul(size_of::<T>())
}

/// Bytes of `capacity` elements of `T`, or none when they overflow.
pub(in crate::application::file_dag) fn checked<T>(capacity: usize) -> Option<usize> {
    capacity.checked_mul(size_of::<T>())
}

/// Bytes of `capacity` elements of `T`, saturating.
pub(in crate::application::file_dag) fn saturating<T>(capacity: usize) -> usize {
    capacity.saturating_mul(size_of::<T>())
}

/// Bytes of a hash table of `capacity` slots holding `T`, with one control byte per slot.
pub(in crate::application::file_dag) fn table<T>(capacity: usize) -> usize {
    capacity.saturating_mul(size_of::<T>() + 1)
}

/// A header value followed by `capacity` elements of `T`, or none when they overflow.
pub(in crate::application::file_dag) fn headed<H, T>(capacity: usize) -> Option<usize> {
    size_of::<H>().checked_add(checked::<T>(capacity)?)
}

/// Arc allocation header and payload alignment, excluding separately owned vectors.
pub(in crate::application::file_dag) const fn arc_bytes<T>() -> usize {
    size_of::<T>() + 2 * size_of::<usize>() + align_of::<T>() - 1
}

/// An Arc-shared vector of row positions: header, vector header and reserved room.
pub(in crate::application::file_dag) fn positions(rows: &Vec<u32>) -> usize {
    size_of::<Vec<u32>>() + 2 * size_of::<usize>() + saturating::<u32>(rows.capacity())
}

/// Like `positions`, but none when the reserved room overflows.
pub(in crate::application::file_dag) fn positions_checked(rows: &Vec<u32>) -> Option<usize> {
    (size_of::<Vec<u32>>() + 2 * size_of::<usize>()).checked_add(checked::<u32>(rows.capacity())?)
}
