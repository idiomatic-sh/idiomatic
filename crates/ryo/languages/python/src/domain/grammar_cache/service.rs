//! Move grammar results across one reset without retaining a corpus of input strings.
use super::types::{Entry, GrammarMemo};
/// Reusable storage admission policy per optional grammar index (four indexes total).
pub(crate) const BUDGET: usize = 16 * 1024;
/// Half of the index allowance is available to content memo entries and assembly scratch.
pub(crate) const MEMO_BUDGET: usize = BUDGET / 2;
/// Each index has at most four reset source arenas; together they use the other half.
pub(crate) const ARENA_BUDGET: usize = BUDGET / 8;
/// Conservative map accounting: buckets plus control bytes, rounded upward twofold.
/// Allocator bookkeeping/rounding is implementation dependent; this is a capacity budget.
#[must_use]
pub(crate) fn retained<V>(memo: &GrammarMemo<V>) -> usize {
    memo.bytes
        .saturating_add(memo.scratch.capacity())
        .saturating_add(
            memo.entries
                .capacity()
                .saturating_mul(size_of::<(String, Entry<V>)>().saturating_add(1))
                .saturating_mul(2),
        )
}
/// Discard unmatched previous-source contents; retain only a budgeted empty table/scratch.
pub(crate) fn empty<V>(memo: &mut GrammarMemo<V>) {
    memo.entries.clear();
    memo.bytes = 0;
    if retained(memo) > MEMO_BUDGET {
        memo.entries = std::collections::HashMap::new();
        if memo.scratch.capacity() > ARENA_BUDGET {
            memo.scratch = String::new();
        }
    }
}
/// Whether a current index was produced by this exact parser implementation and dialect.
#[must_use]
pub(crate) fn is_grammar<P, V>(memo: &GrammarMemo<V>, dialect: &'static str) -> bool {
    memo.grammar == (std::any::type_name::<P>(), dialect)
}
/// A parser implementation change invalidates reuse, even for identical text.
pub(crate) fn grammar<P, V>(memo: &mut GrammarMemo<V>, dialect: &'static str) {
    let identity = (std::any::type_name::<P>(), dialect);
    if memo.grammar != identity {
        empty(memo);
        memo.grammar = identity;
    }
}
/// Admit an owned result only when all its key/result capacity fits the reusable budget.
pub(crate) fn admit<V>(memo: &mut GrammarMemo<V>, key: String, value: V, child_bytes: usize) {
    let bytes = key.capacity().saturating_add(child_bytes);
    if bytes > MEMO_BUDGET || retained(memo).saturating_add(bytes) > MEMO_BUDGET {
        return;
    }
    if memo.entries.try_reserve(1).is_err() {
        return;
    }
    if retained(memo).saturating_add(bytes) > MEMO_BUDGET {
        empty(memo);
        return;
    }
    let previous = memo.entries.insert(key, Entry { value, bytes });
    if let Some(previous) = previous {
        memo.bytes = memo.bytes.saturating_sub(previous.bytes);
    }
    memo.bytes = memo.bytes.saturating_add(bytes);
}
/// Transfer a content-identical value into the new file's active index.
pub(crate) fn take<V>(memo: &mut GrammarMemo<V>, key: &str) -> Option<(String, V)> {
    let (key, entry) = memo.entries.remove_entry(key)?;
    memo.bytes = memo.bytes.saturating_sub(entry.bytes);
    Some((key, entry.value))
}
/// Keep only modest assembly capacity; a large input is active data, not a permanent pool.
pub(crate) fn scratch<V>(memo: &mut GrammarMemo<V>, mut text: String) {
    text.clear();
    if text.capacity() <= ARENA_BUDGET
        && retained(memo).saturating_add(text.capacity()) <= MEMO_BUDGET
    {
        memo.scratch = text;
    }
}
/// Drop oversized active vector storage at reset; small output arenas remain reusable.
pub(crate) fn reset<T>(values: &mut Vec<T>) {
    values.clear();
    if values.capacity().saturating_mul(size_of::<T>()) > ARENA_BUDGET {
        *values = Vec::new();
    }
}
/// Conservative retained capacity for the pinned `IndexMap`'s two independent buffers.
/// Dense entries include a stored hash; the second term covers the index/control table.
/// Our growth path is only `try_reserve(1)`, which rounds dense storage to index capacity.
/// Factors allow bucket load, alignment and growth headroom; allocator rounding is estimated.
#[must_use]
pub(crate) fn ordered_bytes<K, V>(map: &indexmap::IndexMap<K, V>) -> usize {
    let capacity = map.capacity();
    let dense = capacity
        .saturating_mul(size_of::<(usize, K, V)>())
        .saturating_mul(2);
    let indices = capacity
        .saturating_mul(size_of::<usize>().saturating_add(1))
        .saturating_mul(4);
    if capacity == 0 {
        0
    } else {
        dense.saturating_add(indices).saturating_add(32)
    }
}
/// Reserve both ordered-map buffers; on failure discard a possibly partly grown hash index.
pub(crate) fn reserve<K, V>(map: &mut indexmap::IndexMap<K, V>) -> bool
where
    K: Eq + std::hash::Hash,
{
    if map.try_reserve(1).is_ok() {
        true
    } else {
        *map = indexmap::IndexMap::new();
        false
    }
}
/// Drop oversized source lookup storage after moving its owned entries into the memo.
pub(crate) fn reset_map<K, V>(map: &mut indexmap::IndexMap<K, V>) {
    if ordered_bytes(map) > ARENA_BUDGET {
        *map = indexmap::IndexMap::new();
    }
}
