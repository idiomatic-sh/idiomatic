//! Plain data retained by the lazy Python constant-key reader.

use crate::domain::constant_numbers::types::{NumberKey, NumericState};
use ryo::languages::{Position, Symbol};

/// A literal category whose equality remains structural in this relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum LiteralKind {
    /// An integer outside the supported exact numeric range.
    Integer,
    /// A decoded string.
    String,
    /// Parser-decoded bytes.
    Bytes,
    /// The unique `None` literal.
    Null,
    /// The unique ellipsis literal.
    Ellipsis,
}

/// One full key part. Tags distinguish numbers, structural literals and tuple boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum KeyWord {
    /// A normalized Python numeric value.
    Number(NumberKey),
    /// A structural literal category and its canonical parser-owned value, when it has one.
    Literal(LiteralKind, Option<Symbol>),
    /// Start of a tuple key.
    Tuple,
    /// A child key's original representative, already canonicalized in this file.
    Child(Position),
    /// End of a tuple key.
    EndTuple,
}

/// An index into the retained collision-chain records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyRecordIndex(pub usize);

/// A position in the retained key-word arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyWordIndex(pub usize);

/// The words of one complete key in the retained arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WordRange {
    /// The first word of the key.
    pub start: KeyWordIndex,
    /// One past the last word of the key.
    pub end: KeyWordIndex,
}

/// Whether an occurrence is a synthetic literal segment inside an interpolated string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fragment {
    /// A literal segment of an interpolated string.
    Interpolated,
    /// An ordinary occurrence.
    Standalone,
}

/// A complete key, its original representative and the prior key in the same hash bucket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyRecord {
    /// The complete key in the retained arena.
    pub words: WordRange,
    /// An original syntax node representing this value class.
    pub representative: Position,
    /// Another full key in the bucket, compared after hashing.
    pub previous: Option<KeyRecordIndex>,
}

/// Whether a source's key index is still current and complete.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum KeyState {
    /// The current source has not requested keys.
    #[default]
    Fresh,
    /// The retained keys cover every supported occurrence.
    Complete,
    /// Some syntax, payload or checked storage could not be represented.
    Partial,
}

/// One constant expression's canonical representative and optional numeric payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyClass {
    /// The first original occurrence with this full key.
    pub representative: Position,
    /// Numeric data used by bounded unary and complex addition/subtraction normalization.
    pub number: Option<NumericState>,
}

/// Capacity-retaining file-local full keys. Hashes choose candidates; complete word slices decide equality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RetainedKeyIndex<Class, Word, Digest: std::hash::Hash + Eq, State> {
    /// One supported canonical class per source element position.
    pub classes: Vec<Option<Class>>,
    /// Full keys retained once per distinct value class.
    pub words: Vec<Word>,
    /// Canonical representatives and collision chains.
    pub records: Vec<KeyRecord>,
    /// Hash bucket heads; equality never relies on a hash alone.
    pub buckets: std::collections::HashMap<Digest, KeyRecordIndex>,
    /// Reusable full-key construction buffer.
    pub scratch: Vec<Word>,
    /// Whether this index belongs to the current source revision.
    pub state: State,
}

impl<Class, Word, Digest: std::hash::Hash + Eq, State: Default> Default
    for RetainedKeyIndex<Class, Word, Digest, State>
{
    fn default() -> Self {
        Self {
            classes: Vec::new(),
            words: Vec::new(),
            records: Vec::new(),
            buckets: std::collections::HashMap::new(),
            scratch: Vec::new(),
            state: State::default(),
        }
    }
}

/// The retained index of normalized constant-expression keys.

/// Whether a candidate expression belongs to the caller's semantic projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyVisibility {
    /// The source occurrence participates in the relation.
    Visible,
    /// The source occurrence is outside the selected semantic syntax.
    Hidden,
}

impl From<bool> for KeyVisibility {
    fn from(visible: bool) -> Self {
        if visible { Self::Visible } else { Self::Hidden }
    }
}

/// The numeric payload retained alongside a successfully constructed full key.
pub(crate) struct BuiltKey {
    /// Numeric value when this is a primitive or normalized numeric expression.
    pub number: Option<NumericState>,
}

/// The hash that selects a bucket of candidate keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct KeyHash(pub u64);
