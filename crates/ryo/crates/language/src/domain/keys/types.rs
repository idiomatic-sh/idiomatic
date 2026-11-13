//! The types of the `keys` concept: plain data, public fields and derives. A file's structural keys, retained between files with
//! their capacity, each complete key once.

use ryo_relations::domain::relations::types::{FieldId, KindBits, Position, PropertyId};
use ryo_relations::domain::terms::types::Symbol;
use std::collections::HashMap;

/// An index into the retained collision-chain records.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::types::KeyRecordIndex;
/// assert_eq!(KeyRecordIndex(0), KeyRecordIndex(0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyRecordIndex(pub usize);

/// A position in the retained key-word arena.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::types::KeyWordIndex;
/// assert_eq!(KeyWordIndex(2).0, 2);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyWordIndex(pub usize);

/// The words of one complete key in the retained arena.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::types::{KeyWordIndex, WordRange};
/// let range = WordRange { start: KeyWordIndex(0), end: KeyWordIndex(3) };
/// assert_eq!(range.end.0 - range.start.0, 3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordRange {
    /// The first word of the key.
    pub start: KeyWordIndex,
    /// One past the last word of the key.
    pub end: KeyWordIndex,
}

/// The record a bucket held before a record joined it: the next candidate with the same digest, or none.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::types::{BucketLink, KeyRecordIndex};
/// assert_ne!(BucketLink::Previous(KeyRecordIndex(0)), BucketLink::First);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BucketLink {
    /// Another full key in the bucket, compared after hashing.
    Previous(KeyRecordIndex),
    /// The first key of its bucket.
    First,
}

/// A complete key, the first occurrence that had it, and the record before it in its bucket.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::types::{BucketLink, KeyRecord, KeyWordIndex, WordRange};
/// use ryo_relations::domain::relations::types::Position;
/// let record = KeyRecord {
///     words: WordRange { start: KeyWordIndex(0), end: KeyWordIndex(1) },
///     representative: Position(4),
///     previous: BucketLink::First,
/// };
/// assert_eq!(record.representative, Position(4));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyRecord {
    /// The complete key in the retained arena.
    pub words: WordRange,
    /// The first occurrence with this key, which stands for every occurrence with it.
    pub representative: Position,
    /// The record before this one in its bucket.
    pub previous: BucketLink,
}

/// The digest that chooses a bucket of candidate keys; it never decides equality.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::types::KeyDigest;
/// assert_eq!(KeyDigest(7), KeyDigest(7));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyDigest(pub u64);

/// Whether a file's index belongs to its current source, and whether it covers every occurrence.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::types::KeyState;
/// assert_eq!(KeyState::default(), KeyState::Fresh);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyState {
    /// The current source has not requested keys.
    #[default]
    Fresh,
    /// The retained keys cover every supported occurrence.
    Complete,
    /// Some syntax, payload or storage could not be represented.
    Partial,
}

/// One word of an expression's structural key. Every word is tagged, so a child's class is never mistaken for literal text.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::types::ShapeWord;
/// use ryo_relations::domain::terms::types::Symbol;
/// assert_ne!(ShapeWord::Identifier(Symbol(1)), ShapeWord::Literal(Symbol(1)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapeWord {
    /// The parsed node kind.
    Kind(KindBits),
    /// A child's role and the class of the child.
    Child(FieldId, Position),
    /// A significant operator or keyword, and its text.
    Token(KindBits, Symbol),
    /// A canonical identifier.
    Identifier(Symbol),
    /// A decoded literal value.
    Literal(Symbol),
    /// A typed scalar property the parser captured.
    Property(PropertyId, Symbol),
}

/// A file's complete keys, their capacity kept between files: one class per element position, each distinct key's words once,
/// and the records and buckets that find an equal key again.
///
/// # Examples
/// ```
/// use ryo_language::domain::keys::types::{KeyState, RetainedKeyIndex, ShapeWord};
/// use ryo_relations::domain::relations::types::Position;
/// let index = RetainedKeyIndex::<Position, ShapeWord>::default();
/// assert_eq!(index.state, KeyState::Fresh);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedKeyIndex<Class, Word> {
    /// One class per source element position, where the occurrence has one.
    pub classes: Vec<Option<Class>>,
    /// Full keys, retained once per distinct class.
    pub words: Vec<Word>,
    /// Representatives and collision chains.
    pub records: Vec<KeyRecord>,
    /// Bucket heads; equality never relies on a digest alone.
    pub buckets: HashMap<KeyDigest, KeyRecordIndex>,
    /// The key being built, its buffer reused.
    pub scratch: Vec<Word>,
    /// Whether this index belongs to the current source revision.
    pub state: KeyState,
}

impl<Class, Word> Default for RetainedKeyIndex<Class, Word> {
    fn default() -> Self {
        Self {
            classes: Vec::new(),
            words: Vec::new(),
            records: Vec::new(),
            buckets: HashMap::new(),
            scratch: Vec::new(),
            state: KeyState::default(),
        }
    }
}
