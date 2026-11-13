//! The `keys` service: free functions named by verb that clear a file's key index, append to the key being built, intern it,
//! and read an occurrence's class, with no IO and no ports. Which words make a key is the language's: these functions never
//! read a tree.

use super::types::{
    BucketLink, KeyDigest, KeyRecord, KeyRecordIndex, KeyState, KeyWordIndex, RetainedKeyIndex,
    WordRange,
};
use ryo_relations::domain::relations::types::Position;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher as _};

/// Empties the index for the next source while keeping every allocation, and marks it fresh.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::service::clear_keys;
/// use ryo_language::domain::keys::types::{KeyState, RetainedKeyIndex, ShapeWord};
/// use ryo_relations::domain::relations::types::Position;
/// let mut index = RetainedKeyIndex::<Position, ShapeWord>::default();
/// index.state = KeyState::Complete;
/// clear_keys(&mut index);
/// assert_eq!(index.state, KeyState::Fresh);
/// ```
pub fn clear_keys<Class, Word>(index: &mut RetainedKeyIndex<Class, Word>) {
    index.classes.clear();
    index.words.clear();
    index.records.clear();
    index.buckets.clear();
    index.scratch.clear();
    index.state = KeyState::Fresh;
}

/// The class of the occurrence at `at`, when it has one.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::service::key_class_at;
/// use ryo_language::domain::keys::types::{RetainedKeyIndex, ShapeWord};
/// use ryo_relations::domain::relations::types::Position;
/// let mut index = RetainedKeyIndex::<Position, ShapeWord>::default();
/// index.classes = vec![None, Some(Position(1))];
/// assert_eq!(key_class_at(&index, Position(1)), Some(Position(1)));
/// assert_eq!(key_class_at(&index, Position(0)), None);
/// ```
#[must_use]
pub fn key_class_at<Class, Word>(
    index: &RetainedKeyIndex<Class, Word>,
    at: Position,
) -> Option<Class>
where
    Class: Copy,
{
    let Ok(slot) = usize::try_from(at.0) else {
        return None;
    };
    index.classes.get(slot).copied().flatten()
}

/// Appends one word to the key being built; when its buffer cannot grow, marks the index partial and answers `None`.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::service::push_key_word;
/// use ryo_language::domain::keys::types::{RetainedKeyIndex, ShapeWord};
/// use ryo_relations::domain::relations::types::Position;
/// use ryo_relations::domain::terms::types::Symbol;
/// let mut index = RetainedKeyIndex::<Position, ShapeWord>::default();
/// assert_eq!(push_key_word(&mut index, ShapeWord::Identifier(Symbol(1))), Some(()));
/// assert_eq!(index.scratch.len(), 1);
/// ```
pub fn push_key_word<Class, Word>(
    index: &mut RetainedKeyIndex<Class, Word>,
    word: Word,
) -> Option<()> {
    if index.scratch.try_reserve(1).is_err() {
        index.state = KeyState::Partial;
        return None;
    }
    index.scratch.push(word);
    Some(())
}

/// Interns the key built in `scratch` for the occurrence at `at`, and answers its class: the first occurrence whose complete
/// key is equal, or `at` itself. A digest only chooses the bucket; a collision compares the full words. When the index cannot
/// grow, the occurrence stands for itself and the index is marked partial.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::service::intern_key;
/// use ryo_language::domain::keys::types::{RetainedKeyIndex, ShapeWord};
/// use ryo_relations::domain::relations::types::Position;
/// use ryo_relations::domain::terms::types::Symbol;
/// let mut index = RetainedKeyIndex::<Position, ShapeWord>::default();
/// index.scratch.push(ShapeWord::Identifier(Symbol(1)));
/// assert_eq!(intern_key(&mut index, Position(2)), Position(2));
/// assert_eq!(intern_key(&mut index, Position(5)), Position(2));
/// ```
pub fn intern_key<Class, Word>(index: &mut RetainedKeyIndex<Class, Word>, at: Position) -> Position
where
    Word: Hash + Eq + Clone,
{
    let mut hasher = DefaultHasher::new();
    index.scratch.hash(&mut hasher);
    let digest = KeyDigest(hasher.finish());
    let mut candidate = index.buckets.get(&digest).copied();
    while let Some(place) = candidate {
        let Some(record) = index.records.get(place.0) else {
            break;
        };
        if index.words.get(record.words.start.0..record.words.end.0)
            == Some(index.scratch.as_slice())
        {
            return record.representative;
        }
        candidate = match record.previous {
            BucketLink::Previous(previous) => Some(previous),
            BucketLink::First => None,
        };
    }
    if index.words.try_reserve(index.scratch.len()).is_err()
        || index.records.try_reserve(1).is_err()
        || index.buckets.try_reserve(1).is_err()
    {
        index.state = KeyState::Partial;
        return at;
    }
    let start = index.words.len();
    index.words.extend_from_slice(&index.scratch);
    let place = KeyRecordIndex(index.records.len());
    index.records.push(KeyRecord {
        words: WordRange {
            start: KeyWordIndex(start),
            end: KeyWordIndex(index.words.len()),
        },
        representative: at,
        previous: index
            .buckets
            .get(&digest)
            .copied()
            .map_or(BucketLink::First, BucketLink::Previous),
    });
    let _previous_bucket_head = index.buckets.insert(digest, place);
    at
}
