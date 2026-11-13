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

/// Whether an occurrence is a synthetic literal segment inside an interpolated string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConstantKeyFragment {
    /// A literal segment of an interpolated string.
    Interpolated,
    /// An ordinary occurrence.
    Standalone,
}

/// One constant expression's canonical representative and optional numeric payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyClass {
    /// The first original occurrence with this full key.
    pub representative: Position,
    /// Numeric data used by bounded unary and complex addition/subtraction normalization.
    pub number: Option<NumericState>,
}

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
