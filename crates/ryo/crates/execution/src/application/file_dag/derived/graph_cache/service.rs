//! Complete compact graph/domain acquisition under one unchanged per-file byte allowance.

use crate::application::file_dag::derived::backend::CacheBackend;

/// Root and nearest answers cannot silently grow into a retained all-pairs closure.
pub(crate) const MEMO_LIMIT: usize = 2 * 1024 * 1024;

/// Replacement metadata is allocated separately, so refusal never grows retained capacity.
pub(crate) fn slot<T>(entries: &Vec<T>) -> Option<(Option<Vec<T>>, usize)> {
    if entries.len() < entries.capacity() {
        return Some((None, 0));
    }
    let mut replacement = Vec::new();
    replacement
        .try_reserve_exact(entries.len().checked_add(1)?)
        .ok()?;
    let more = replacement
        .capacity()
        .checked_sub(entries.capacity())?
        .checked_mul(size_of::<T>())?;
    Some((Some(replacement), more))
}

/// Move existing entries only after complete admission and byte accounting succeeds.
pub(crate) fn replace<T>(entries: &mut Vec<T>, replacement: Option<Vec<T>>) {
    if let Some(mut replacement) = replacement {
        replacement.append(entries);
        *entries = replacement;
    }
}

/// Account all retained payload under the original total; no fixed allowance is stranded.
pub(in crate::application::file_dag::derived) fn room(
    cache: &CacheBackend,
    cost: usize,
) -> Option<usize> {
    let total = cache.bytes.checked_add(cost)?;
    (total <= cache.limit).then_some(total)
}
