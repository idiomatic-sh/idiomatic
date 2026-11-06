//! Move grammar results across one reset without retaining a corpus of input strings.
use super::constants::{
    KEY_CAPACITY, MAP_CAPACITY, MEMO_ENTRIES, SCRATCH_CAPACITY, VECTOR_CAPACITY,
};
use super::types::{
    DialectName, Entry, GrammarMemo, GrammarState, Lookup, MemoGrammar, MemoKey, ParserName,
    Reservation, Scratch, Taken,
};

/// Discard unmatched previous-source contents; retain only a bounded empty table and scratch.
pub(crate) fn empty<V>(memo: &mut GrammarMemo<V>) {
    memo.entries.clear();
    if memo.entries.capacity() > MEMO_ENTRIES {
        memo.entries = std::collections::HashMap::new();
    }
    if memo.scratch.0.capacity() > SCRATCH_CAPACITY {
        memo.scratch = Scratch::default();
    }
}

fn identity<P>(dialect: DialectName) -> MemoGrammar {
    MemoGrammar {
        parser: ParserName(std::any::type_name::<P>()),
        dialect,
    }
}

/// Whether a current index was produced by this exact parser implementation and dialect.
#[must_use]
pub(crate) fn grammar_state<P, V>(memo: &GrammarMemo<V>, dialect: DialectName) -> GrammarState {
    if memo.grammar == identity::<P>(dialect) {
        GrammarState::Current
    } else {
        GrammarState::Stale
    }
}

/// A parser implementation change invalidates reuse, even for identical text.
pub(crate) fn grammar<P, V>(memo: &mut GrammarMemo<V>, dialect: DialectName) {
    let identity = identity::<P>(dialect);
    if memo.grammar != identity {
        empty(memo);
        memo.grammar = identity;
    }
}

/// Admit an owned result only while the memo has room and its key is modest.
pub(crate) fn admit<V>(memo: &mut GrammarMemo<V>, key: MemoKey, value: V) {
    if key.0.capacity() > KEY_CAPACITY || memo.entries.len() >= MEMO_ENTRIES {
        return;
    }
    if memo.entries.try_reserve(1).is_err() {
        return;
    }
    let _previous = memo.entries.insert(key, Entry { value });
}

/// Transfer a content-identical value into the new file's active index.
pub(crate) fn take<V>(memo: &mut GrammarMemo<V>, key: Lookup<'_>) -> Option<Taken<V>> {
    let (key, entry) = memo.entries.remove_entry(key.0)?;
    Some(Taken {
        key,
        value: entry.value,
    })
}

/// Keep only modest assembly capacity; a large input is active data, not a permanent pool.
pub(crate) fn scratch<V>(memo: &mut GrammarMemo<V>, mut text: Scratch) {
    text.0.clear();
    if text.0.capacity() <= SCRATCH_CAPACITY {
        memo.scratch = text;
    }
}

/// Drop oversized active vector storage at reset; small output arenas remain reusable.
pub(crate) fn reset<T>(values: &mut Vec<T>) {
    values.clear();
    if values.capacity() > VECTOR_CAPACITY {
        *values = Vec::new();
    }
}

/// Reserve both ordered-map buffers; on failure discard a possibly partly grown hash index.
pub(crate) fn reserve<K, V>(map: &mut indexmap::IndexMap<K, V>) -> Reservation
where
    K: Eq + std::hash::Hash,
{
    if map.try_reserve(1).is_ok() {
        Reservation::Granted
    } else {
        *map = indexmap::IndexMap::new();
        Reservation::Refused
    }
}

/// Drop oversized source lookup storage after moving its owned entries into the memo.
pub(crate) fn reset_map<K, V>(map: &mut indexmap::IndexMap<K, V>) {
    if map.capacity() > MAP_CAPACITY {
        *map = indexmap::IndexMap::new();
    }
}
