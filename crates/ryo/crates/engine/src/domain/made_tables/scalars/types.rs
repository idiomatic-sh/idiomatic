//! The word of a cell together with its bound kind.
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::CellKind;
/// One evaluated scalar. Its kind is retained even when its word denotes a missing value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackedScalar {
    /// Logical type of the packed word.
    pub kind: CellKind,
    /// Payload bits, or the missing-cell sentinel.
    pub word: CellWord,
}

/// Rendered scalar spelling, borrowing the dictionary whenever the value has an entry.
pub struct ScalarSpelling<'a>(pub std::borrow::Cow<'a, [u8]>);

/// A borrowed scalar as Unicode text during natural ordering.
#[derive(Clone, Copy)]
pub struct CollationText<'a>(pub &'a str);
