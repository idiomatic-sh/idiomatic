//! Collision-safe structural keys and retained indexing storage.
use crate::domain::{labels::types::Field, syntax::types::Kind};
use ryo::languages::{Position, PropertyId, Symbol};
/// Every component is tagged; a child identity cannot be mistaken for literal text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ShapeWord {
    /// The parsed node kind.
    Kind(Kind),
    /// A semantic field and its child class.
    Child(Field, Position),
    /// A significant operator or keyword.
    Token(Kind, Symbol),
    /// Canonical lexical identifier.
    Identifier(Symbol),
    /// A decoded literal segment, merged across implicit concatenation.
    Literal(Symbol),
    /// A non-string literal keyword, kept distinct from decoded text.
    LiteralKeyword(Symbol),
    /// A typed scalar property captured by the parser.
    Property(PropertyId, Symbol),
}
/// A hash chooses candidates only; full stored keys establish equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ShapeHash(pub u64);
/// An index into the retained class records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShapeRecordIndex(pub usize);
/// A canonical key and an original syntax representative.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ShapeRecord {
    /// Its complete key in the shared arena.
    pub words: std::ops::Range<usize>,
    /// A real syntax occurrence, not a truncated hash.
    pub representative: Position,
    /// Another key with the same hash, compared in full on lookup.
    pub previous: Option<ShapeRecordIndex>,
}
/// Whether the retained index belongs to a complete current tree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ShapeState {
    #[default]
    Fresh,
    Complete,
    Partial,
}
/// Storage is kept across source edits; no index is built until requested.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ExpressionShapeIndex {
    /// One optional canonical representative per syntax element.
    pub classes: Vec<Option<Position>>,
    /// Full keys, stored once for each unique class.
    pub words: Vec<ShapeWord>,
    /// Canonical classes and collision chains.
    pub records: Vec<ShapeRecord>,
    /// Hash bucket heads; equality never relies on this key alone.
    pub buckets: std::collections::HashMap<ShapeHash, ShapeRecordIndex>,
    /// One reusable key builder.
    pub scratch: Vec<ShapeWord>,
    /// Current parse validity.
    pub state: ShapeState,
}
