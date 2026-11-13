//! The `keys` service: free functions named by verb that clear a file's key index, append to the key being built, intern it,
//! and read an occurrence's class, with no IO and no ports. Which words make a key is the language's: these functions never
//! read a tree.

use super::types::RetainedKeyIndex;
use ryo_relations::domain::relations::types::Position;
use std::hash::Hash;

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
    let _ = (index);
    unimplemented!("the move of the key index lands with the next commit")
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
    let _ = (index, at);
    unimplemented!("the move of the key index lands with the next commit")
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
    let _ = (index, word);
    unimplemented!("the move of the key index lands with the next commit")
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
    let _ = (index, at);
    unimplemented!("the move of the key index lands with the next commit")
}
