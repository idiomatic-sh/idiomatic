//! Complete compact graph/domain acquisition under one unchanged per-file byte allowance.

use super::reserve::types::Reserve;
use crate::application::file_dag::accounting::ledger::types::{ByteCount, SlotCount};
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;

/// Root and nearest answers cannot silently grow into a retained all-pairs closure.
pub(crate) const MEMO_LIMIT: usize = 2 * 1024 * 1024;

/// Replacement metadata is allocated separately, so refusal never grows retained capacity.
pub(crate) fn slot<T>(entries: &Vec<T>) -> Option<Reserve<T>> {
    if entries.len() < entries.capacity() {
        return Some(Reserve {
            replacement: None,
            metadata: ByteCount(0),
        });
    }
    let mut replacement = Vec::new();
    if replacement
        .try_reserve_exact(entries.len().checked_add(1)?)
        .is_err()
    {
        return None;
    }
    let metadata = crate::application::file_dag::accounting::ledger::capacity::service::checked::<T>(
        SlotCount(replacement.capacity().checked_sub(entries.capacity())?),
    )?;
    Some(Reserve {
        replacement: Some(replacement),
        metadata,
    })
}

/// Move existing entries only after complete admission and byte accounting succeeds.
pub(crate) fn replace<T>(entries: &mut Vec<T>, replacement: Option<Vec<T>>) {
    if let Some(mut replacement) = replacement {
        replacement.append(entries);
        *entries = replacement;
    }
}

/// Account all retained payload under the original total; no fixed allowance is stranded.
pub(crate) fn room(cache: &CacheBackend, cost: ByteCount) -> Option<ByteCount> {
    let total = cache.bytes.0.checked_add(cost.0)?;
    (total <= cache.limit.0).then_some(ByteCount(total))
}
