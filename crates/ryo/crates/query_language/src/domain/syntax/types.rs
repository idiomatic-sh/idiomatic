//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. The operations of the language and the arguments they take are enums;
//! their outside text is in `constants.rs`.

use ryo_values::domain::plans::types::QueryOperator;

/// What an argument of a call is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArgumentRole {
    /// It names a relation or a step the call reads.
    Input,
    /// It is anything else the call takes.
    Other,
}

/// The arguments a call can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ArgumentSlot {
    /// Constant replacement text.
    Replacement,
    /// Left numeric ordering column.
    LeftOrder,
    /// Right numeric ordering column.
    RightOrder,
    /// `from`.
    From,
    /// `columns`.
    Columns,
    /// `values`.
    Values,
    /// `left`.
    Left,
    /// `right`.
    Right,
    /// `left-on`.
    LeftOn,
    /// `right-on`.
    RightOn,
    /// `output`.
    Output,
    /// `keys`.
    Keys,
    /// `distinct`.
    Distinct,
    /// `count`.
    Count,
    /// `digest`.
    Digest,
    /// `path`.
    Path,
    /// `pattern`.
    Pattern,
    /// `captures`.
    Captures,
    /// `text`.
    Text,
    /// `word`.
    Word,
    /// `relation`.
    Relation,
    /// `direction`.
    Direction,
    /// `domain`.
    Domain,
    /// `edges`.
    Edges,
    /// `source`.
    Source,
    /// `destination`.
    Destination,
    /// `column`.
    Column,
    /// `sets`.
    Sets,
}

/// What the language says about one operation: the arguments that name an input, in the order
/// the engine reads its inputs, and the other arguments it cannot be written without.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OperationShape {
    /// The operation.
    pub operator: QueryOperator,
    /// The arguments that name an input.
    pub inputs: &'static [ArgumentSlot],
    /// The other required arguments.
    pub required: &'static [ArgumentSlot],
}
