//! Capacity helpers for bounded retained metadata. Payload ownership is ledger-based.
/// Account actual capacity rather than live length.
pub(crate) fn vector<T>(values: &Vec<T>) -> usize {
    values.capacity().saturating_mul(size_of::<T>())
}

/// Arc allocation header and payload alignment, excluding separately owned vectors.
pub(crate) const fn arc_bytes<T>() -> usize {
    size_of::<T>() + 2 * size_of::<usize>() + align_of::<T>() - 1
}
